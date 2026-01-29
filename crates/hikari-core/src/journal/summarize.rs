pub mod error;
use crate::journal::summarize::error::SummarizeError;
use crate::llm_config::LlmConfig;

use crate::openai::{CallConfig, FunctionResponse, openai_call_function_with_timeout};
use base64::Engine;
use chrono::{DateTime, FixedOffset, NaiveDateTime, TimeZone, Utc};

use hikari_db::journal;
use hikari_db::journal::journal_summary;
use hikari_model::journal::MetaJournalEntryWithMetaContent;
use hikari_utils::date::get_day_bounds;

use async_openai::types::{
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
};
use sea_orm::prelude::Uuid;
use sea_orm::{ConnectionTrait, TransactionTrait};
use serde_derive::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::Digest;
use sha2::Sha256;
use std::collections::HashMap;
use std::error::Error;
use std::ops::Add;
use std::sync::{Arc, LazyLock};
use std::time::Duration;

use tokio::sync::Mutex;
use tracing::{Instrument, instrument};

use crate::openai::error::OpenAiError;
use utoipa::ToSchema;

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct TopicSummary {
    pub topic: String,
    pub summary: String,
}

impl TopicSummary {
    fn fix_escapes(&mut self) {
        self.topic = html_escape::decode_html_entities(&self.topic).to_string();
        self.summary = html_escape::decode_html_entities(&self.summary).to_string();
    }
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct SummaryFunctionResponse {
    pub summary: String,
    #[serde(default)]
    pub topic_summaries: Vec<TopicSummary>,
}

impl FunctionResponse for SummaryFunctionResponse {
    fn function_name() -> &'static str {
        "summary"
    }

    fn function_description() -> &'static str {
        "Gibt die Zusammenfassung des Journals zurück. Die Zusammenfassung ist in zwei Komponenten aufgeteilt. Die erste Komponente ist die Gesamtzusammenfassung der Journaleinträge. Die zweite Komponente sind die Zusammenfassungen der Kernthemen."
    }

    fn function_definition() -> Value {
        json! (
            {
              "type": "object",
              "properties": {
                "summary": {
                  "type": "string",
                  "description": "Gesamtzusammenfassung in der der Nutzer geduzt wird (zweite Person). Die Zusammenfassung sollte maximal 6 Sätze lang sein und sollte, wenn möglich, nicht mit den Kernthemen überlappen. Beispiele: \"Die letzten Tage hattest du viel Stress. Dennoch ging es dir überwiegend gut auch wenn du einen Tag hattest an dem deine Stimmung schlecht war. Beim Lernen konntest du dich dennoch gut konzentrieren.\""
                },
                "topic_summaries": {
                  "type": "array",
                  "items": {
                    "type": "object",
                    "properties": {
                      "topic": {
                        "type": "string",
                        "description": "Der name des Kernthemas in wenigen Worten. Beispiele: \"Stress\", \"Motivation beim Lernen\", \"Freunde\""
                      },
                      "summary": {
                        "type": "string",
                        "description": "Zusammenfassung des Kernthemas in ein bis zwei Sätzen. Die Zusammenfassung sollte den Nutzer duzen (in zweiter Person geschrieben sein). Beispiele: \"Du hattest viel Stress und Probleme damit umzugehen\", \"Deine Motivation beim Lernen kam ganz auf das Fach an. Bei Mathe hast du dich Angestrengt, aber für Urheberrecht konntest du dich nicht begeistern.\""
                      }
                    },
                  },
                  "description": "Unterthemen und deren zusammenfassung."
                },
              },
              "required": ["summary"]
            }
        )
    }

    fn fix_escapes(&mut self) {
        self.summary = html_escape::decode_html_entities(&self.summary).to_string();
        self.topic_summaries.iter_mut().for_each(TopicSummary::fix_escapes);
    }
}

pub(crate) fn generate_summary_response(
    user_id: Uuid,
    journal_entries: Vec<hikari_entity::journal::journal_entry::Model>,
    summary: hikari_entity::journal::journal_summary::Model,
    topic_summaries: Vec<hikari_entity::journal::journal_topic::Model>,
) -> SummaryResponse {
    let mut meta_journal_entries = vec![];
    for journal_entry in journal_entries.into_iter().rev() {
        meta_journal_entries.push(MetaJournalEntryWithMetaContent {
            id: journal_entry.id,
            user_id,
            created_at: journal_entry.created_at,
            updated_at: journal_entry.updated_at,
            ..Default::default()
        });
    }
    SummaryResponse {
        summary: Some(SummaryFunctionResponse {
            summary: summary.summary,
            topic_summaries: topic_summaries
                .into_iter()
                .map(|ts| TopicSummary {
                    topic: ts.topic,
                    summary: ts.summary,
                })
                .collect(),
        }),
        journal_entries: meta_journal_entries,
    }
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct SummaryResponse {
    pub summary: Option<SummaryFunctionResponse>,
    pub journal_entries: Vec<MetaJournalEntryWithMetaContent>,
}
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct Key {
    key: [u8; 32],
    date: NaiveDateTime,
}

#[allow(clippy::type_complexity)]
static LOADING_SUMMARIES: LazyLock<Mutex<HashMap<Key, Arc<Mutex<Option<SummaryResponse>>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[instrument(skip_all)]
pub async fn summarize<C: ConnectionTrait + TransactionTrait + Send + Clone + 'static>(
    conn: C,
    user_id: Uuid,
    llm_config: Arc<LlmConfig>,
    timestamp: Option<DateTime<FixedOffset>>,
) -> Result<SummaryResponse, SummarizeError> {
    tracing::info!(%user_id, timestamp = timestamp.as_ref().map_or_else(|| "NONE".to_owned(), DateTime::to_rfc2822), "creating summary");
    let journal_entries_full =
        journal::journal_entry::Query::get_user_journal_full(&conn.clone(), user_id, Some(5)).await?;

    if journal_entries_full.is_empty() {
        return Ok(SummaryResponse {
            summary: None,
            journal_entries: vec![],
        });
    }

    let entry_ids: Vec<_> = journal_entries_full.iter().map(|(entry, ..)| entry.id).collect();

    let timestamp = timestamp.unwrap_or_else(|| Utc::now().fixed_offset());
    let (from_date, to_date) = get_day_bounds(timestamp)?;
    let key = generate_key(&entry_ids);
    let map_key = Key { key, date: from_date };
    let str_key = base64::engine::general_purpose::STANDARD.encode(key);

    let running: Option<Arc<Mutex<Option<SummaryResponse>>>> = {
        let loading = LOADING_SUMMARIES
            .lock()
            .instrument(tracing::info_span!("acquire_loading_lock"))
            .await;
        loading.get(&map_key).cloned()
        // Make sure we unlock the hashmap
    };
    if let Some(running) = running {
        tracing::debug!(%user_id, key = %str_key, "attaching to running summary");
        let summary = running
            .lock()
            .instrument(tracing::info_span!("acquire_running_lock"))
            .await;
        if let Some(summary) = summary.as_ref() {
            tracing::debug!(%user_id, key = %str_key, "returning shared summary");
            return Ok(summary.clone());
        }
        tracing::debug!(%user_id, key = %str_key, "Got summary lock without a result");
    }

    // Have to clone the key because we need to move it into the task
    let str_key_clone = str_key.clone();
    let summary_task = tokio::task::spawn(
        async move {
            let summary = {
                let loading_mutex = Arc::new(Mutex::new(None));
                // We lock the mutex before inserting it to make sure we hold the lock until we can insert a result.
                let mut loading_mutex_value = loading_mutex.lock().await;
                {
                    LOADING_SUMMARIES
                        .lock()
                        .await
                        .insert(map_key.clone(), Arc::clone(&loading_mutex));
                    // Make sure the hashmap is unlocked
                }
                let summary = generate_summary(
                    &conn,
                    &llm_config,
                    journal_entries_full,
                    user_id,
                    &key,
                    timestamp,
                    from_date,
                    to_date,
                )
                .await?;
                tracing::debug!(%user_id, key = %str_key_clone, "storing summary result in shared map");
                *loading_mutex_value = Some(summary.clone());
                // Unlock the loading map value
                summary
            };
            {
                let mut map_guard = LOADING_SUMMARIES
                    .lock()
                    .instrument(tracing::info_span!("acquire_loading_lock"))
                    .await;
                map_guard.remove(&map_key);
                tracing::debug!(%user_id, key = %str_key_clone, remaining = map_guard.len(), "cleaning up shared map");
                // Make sure the hashmap is unlocked
            }
            Result::<_, SummarizeError>::Ok(summary)
        }
        .instrument(tracing::info_span!("generate_summary_task")),
    );
    let summary = summary_task
        .await
        .map_err(|error| {
            tracing::error!(error = &error as &dyn Error, "summary task failed");
            SummarizeError::Other(error.to_string())
        })?
        .map_err(|error| {
            tracing::error!(error = &error as &dyn Error, "failed to generate summary");
            SummarizeError::Other(error.to_string())
        })?;

    tracing::debug!(%user_id, key = %str_key, "returning summary");
    Ok(summary)
}

type SummaryData = (
    hikari_entity::journal::journal_entry::Model,
    Vec<hikari_entity::journal::journal_content::Model>,
    Vec<hikari_entity::tag::Model>,
    Vec<hikari_entity::journal::journal_prompt::Model>,
);

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
async fn generate_summary<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    llm_config: &LlmConfig,
    journal_entries_full: Vec<SummaryData>,
    user_id: Uuid,
    key: &[u8; 32],
    timestamp: DateTime<FixedOffset>,
    from_date: NaiveDateTime,
    to_date: NaiveDateTime,
) -> Result<SummaryResponse, SummarizeError> {
    let str_key = base64::engine::general_purpose::STANDARD.encode(key);
    if let Some((summary, topic_summaries)) =
        journal_summary::Query::find(conn, user_id, key, from_date, to_date).await?
    {
        tracing::debug!(%user_id, timestamp = timestamp.to_rfc2822(), key = %str_key, "returning saved journal summary");
        return Ok(generate_summary_response(
            user_id,
            journal_entries_full.into_iter().map(|(entry, ..)| entry).collect(),
            summary,
            topic_summaries,
        ));
    }

    let mut messages: Vec<ChatCompletionRequestMessage> = vec![
        ChatCompletionRequestSystemMessageArgs::default()
            .content(format!(
                "\
Ich studieren und erstelle ein Lernjournal.
Du bist ein Assistent der mir hilft, indem er eine Zusammenfassung meiner Lernjournal Einträge erstellt.
Als Nächtes folgen die letzten {} Journaleinträge.
",
                journal_entries_full.len()
            ))
            .build()
            .map_err(OpenAiError::from)?
            .into(),
    ];

    // Journal entries are sorted by created_at in descending order
    let mut journal_entries = vec![];
    for (journal_entry, journal_contents, journal_focuses, journal_prompts) in journal_entries_full.into_iter().rev() {
        let time = match days_since(timestamp, &journal_entry.created_at) {
            0 => "Heute".to_owned(),
            1 => "Gestern".to_owned(),
            2 => "Vorgestern".to_owned(),
            days => format!("Vor {days} Tagen"),
        };

        let header = if journal_prompts.is_empty() {
            let insert = journal_entry.title.as_ref().map_or_else(
                || "ich".to_owned(),
                |title| format!("ich einen Journaleintrag mit der Überschrift \"{title}\" erstellt und dazu"),
            );
            format!("{time} habe {insert} Folgendes geschrieben:\n")
        } else {
            let insert = journal_entry.title.as_ref().map_or_else(
                || "ich mir".to_owned(),
                |title| format!("ich einen Journaleintrag mit der Überschrift \"{title}\" erstellt und mir dabei"),
            );

            format!(
                "{time} habe {insert} diese Fragen gestellt:\n{}\n\nDazu habe ich dann Folgendes geschrieben:\n",
                journal_prompts
                    .into_iter()
                    .map(|prompt| prompt.prompt)
                    .collect::<Vec<_>>()
                    .join("\n"),
            )
        };

        let mood_sentence = journal_entry.mood.map_or_else(String::new, |mood| {
            format!("Heute war meine Stimmung {}.\n\n", get_mood_word(mood))
        });

        let focus_sentence = match journal_focuses.iter().map(|f| &f.name).collect::<Vec<_>>().as_slice() {
            [] => String::new(),
            [focus] => format!("Heute lag der Fokus meines Tages auf \"{focus}\"."),
            [focus_a, focus_b] => {
                format!("Heute waren der Fokus meines Tages \"{focus_a}\" und \"{focus_b}\".")
            }
            focuses => format!(
                "Heute war mein Tag auf diese Themen fokussiert: {}.",
                focuses
                    .iter()
                    .map(|f| format!("\"{f}\""))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
        .add("\n\n");

        let body = journal_contents
            .into_iter()
            .map(|journal_content| {
                let mut prompt = journal_content.content;

                if let Some(title) = journal_content.title {
                    prompt = format!("{title}:\n{prompt}");
                }
                prompt
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        let journal_entry_message = format!("{header}{mood_sentence}{focus_sentence}{body}");

        tracing::debug!(%user_id, timestamp = timestamp.to_rfc2822(), key = %str_key, %journal_entry.id, %journal_entry_message, "formatted journal entry for openai");
        messages.push(
            ChatCompletionRequestUserMessageArgs::default()
                .content(journal_entry_message)
                .build()
                .map_err(OpenAiError::from)?
                .into(),
        );

        journal_entries.push(journal_entry);
    }

    messages.push(ChatCompletionRequestSystemMessageArgs::default().content(
            "\
Bitte Duze mich.

Ich würde gerne über meine Vergangenen einträge Reflektieren, deshalb würde ich gerne eine deutsche Zusammenfassung von dir haben.
Also mach bitte zwei Dinge für mich:

1. Bitte generiere aus den Journaleinträgen eine Zusammenfassung für mich. Die Zusammenfassung sollte nicht länger als sechs Sätze sein.
2. Bitte identifiziere bis zu drei Kernthemen in meinen Journaleinträgen. Fasse auch diese Kernthemen für mich zusammen. Duze mich dabei bitte weiterhin. Das Kernthema sollte sich mit wenigen Worten betiteln lassen und die zugehörige zusammenfassung sollten maximal zwei Sätze lang sein.

Benutze den Funktionsaufruf der dir gegeben wurde.
Verwende Valides JSON als Argumente für den Funktionsaufruf.
"
                .to_string(),
        ).build().map_err(OpenAiError::from)?.into());

    tracing::info!(%user_id, timestamp = timestamp.to_rfc2822(), key = %str_key, "sending {} messages to openAI", messages.len());
    let res: SummaryFunctionResponse = openai_call_function_with_timeout(
        llm_config,
        CallConfig::builder()
            .total_timeout(Duration::from_secs(120))
            .iteration_timeout(Duration::from_secs(60))
            .build(),
        messages,
    )
    .await?;

    let (summary, topic_summaries) = journal_summary::Mutation::create(
        conn,
        user_id,
        timestamp.naive_utc(),
        key,
        res.summary,
        res.topic_summaries
            .into_iter()
            .map(|topic| journal_summary::mutation::Topic {
                title: topic.topic,
                summary: topic.summary,
            })
            .collect(),
    )
    .await?;
    tracing::debug!(%user_id, timestamp = timestamp.to_rfc2822(), key = %str_key, "summary created successfully");
    Ok(generate_summary_response(
        user_id,
        journal_entries,
        summary,
        topic_summaries,
    ))
}

fn get_mood_word(mood: f32) -> &'static str {
    // This is a 5 point scale from 0 to 1. So there should be the values
    // 0, 0.25, 0.5, 0.75, 1
    // We match slightly below that to avoid floating point hell
    match mood {
        mood if mood <= 0.249 => "sehr schlecht",
        mood if mood < 0.499 => "schlecht",
        mood if mood < 0.749 => "neutral",
        mood if mood < 0.999 => "sehr gut",
        _ => "ausgezeichnet",
    }
}

fn days_since(timestamp: DateTime<FixedOffset>, other: &DateTime<FixedOffset>) -> i64 {
    let other_with_tz = timestamp.timezone().from_utc_datetime(&other.naive_utc());
    timestamp
        .naive_local()
        .date()
        .signed_duration_since(other_with_tz.naive_local().date())
        .num_days()
}

fn generate_key(entry_ids: &[Uuid]) -> [u8; 32] {
    let mut entry_ids = entry_ids.to_vec();
    entry_ids.sort();
    let mut hasher = Sha256::new();
    for entry_id in entry_ids {
        hasher.update(entry_id);
    }
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, FixedOffset};

    #[test]
    fn test_days_between() {
        let now = DateTime::<FixedOffset>::parse_from_rfc3339("2023-11-03 20:00:00+06:00").unwrap();
        let less_than_a_day_ago = DateTime::<FixedOffset>::parse_from_rfc3339("2023-11-02 21:00:00+01:00").unwrap();
        let not_today_less_than_24h = DateTime::<FixedOffset>::parse_from_rfc3339("2023-11-02 23:00:00+06:00").unwrap();
        let a_day_ago = DateTime::<FixedOffset>::parse_from_rfc3339("2023-11-02 17:00:00+03:00").unwrap();
        let two_days_ago = DateTime::<FixedOffset>::parse_from_rfc3339("2023-11-02 02:00:00+12:00").unwrap();

        assert_eq!(days_since(now, &less_than_a_day_ago), 0);
        assert_eq!(days_since(now, &not_today_less_than_24h), 1);
        assert_eq!(days_since(now, &a_day_ago), 1);
        assert_eq!(days_since(now, &two_days_ago), 2);
    }

    #[test]
    fn test_get_mood_word() {
        assert_eq!(get_mood_word(0.0), "sehr schlecht");
        assert_eq!(get_mood_word(0.25), "schlecht");
        assert_eq!(get_mood_word(0.5), "neutral");
        assert_eq!(get_mood_word(0.75), "sehr gut");
        assert_eq!(get_mood_word(1.0), "ausgezeichnet");
    }

    #[test]
    fn test_fix_escapes() {
        let topic_summary_a = TopicSummary {
            topic: "ts-a-t m&#228;&#223;ig".to_string(),
            summary: "ts-a-s m&#228;&#223;ig".to_string(),
        };
        let topic_summary_b = TopicSummary {
            topic: "ts-b-t m&#228;&#223;ig".to_string(),
            summary: "ts-b-s m&#228;&#223;ig".to_string(),
        };

        let mut summary_response = SummaryFunctionResponse {
            summary: "m&#228;&#223;ig".to_string(),
            topic_summaries: vec![topic_summary_a, topic_summary_b],
        };
        summary_response.fix_escapes();
        assert_eq!(summary_response.summary, "mäßig");
        assert_eq!(summary_response.topic_summaries.first().unwrap().topic, "ts-a-t mäßig");
        assert_eq!(
            summary_response.topic_summaries.first().unwrap().summary,
            "ts-a-s mäßig"
        );
        assert_eq!(summary_response.topic_summaries.get(1).unwrap().topic, "ts-b-t mäßig");
        assert_eq!(summary_response.topic_summaries.get(1).unwrap().summary, "ts-b-s mäßig");
    }
}
