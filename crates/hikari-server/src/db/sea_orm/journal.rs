use crate::routes::api::v0::modules::error::MessagingError;
use csml_engine::data::AsyncDatabase;
use csml_engine::data::models::Message;
use futures::future::{try_join, try_join_all};
use hikari_db::tag;
use hikari_db::util::FlattenTransactionResultExt;
use hikari_entity::journal::journal_content::{ActiveModel as ActiveJournalContent, Entity as JournalContent};
use hikari_entity::journal::journal_entry::{
    ActiveModel as ActiveJournalEntry, Entity as JournalEntry, Model as JournalEntryModel,
};
use hikari_entity::journal::journal_entry_tag::{ActiveModel as ActiveJournalEntryTag, Entity as JournalEntryTag};
use hikari_model::journal::{MetaContent, MetaJournalEntryWithMetaContent};
use hikari_model::tag::Tag;
use hikari_model_tools::convert::FromDbModel;
use num_traits::ToPrimitive;
use sea_orm::prelude::*;
use sea_orm::{IntoActiveValue, TransactionTrait};
use serde_derive::Deserialize;
use std::collections::HashSet;
use std::error::Error;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct JournalResponse {
    payload: serde_json::Value,
    #[serde(rename = "type")]
    type_name: String,
    #[serde(rename = "display-type")]
    display_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct JournalPrompt {
    prompt: String,
}

struct JournalContentData {
    prompt: Option<String>,
    content: String,
    created_at: chrono::DateTime<chrono::FixedOffset>,
    updated_at: chrono::DateTime<chrono::FixedOffset>,
}

struct JournalData {
    title: Option<String>,
    content: Vec<JournalContentData>,
    mood: Option<f32>,
    focus: HashSet<Uuid>,
}

fn seconds_to_string(seconds: u64) -> String {
    let mut f = timeago::Formatter::with_language(timeago::languages::german::German);
    f.ago("");
    if seconds >= 3600 {
        f.num_items(2);
    }
    let d = std::time::Duration::from_secs(seconds);
    f.convert(d)
}

fn create_journal_entities(conversation_id: Uuid, messages: Vec<Message>) -> JournalData {
    let mut mood = None;
    let mut last_prompt = None;
    let mut content = vec![];
    let mut focus = HashSet::new();
    let mut title = None;

    // Messages are returned in reverse order
    for message in messages.into_iter().rev() {
        let message_id = message.id;
        let created_at = message.created_at.fixed_offset();
        let updated_at = message.updated_at.fixed_offset();
        let Some(payload_content) = message.payload.content else {
            tracing::trace!(%conversation_id, message_id = %message.id, "missing message content");
            // If we don't have any content continue - there is nothing to parse
            continue;
        };
        match message.payload.content_type.as_str() {
            "payload" => {
                let Ok(JournalResponse {
                    type_name,
                    payload,
                    display_type,
                }) = serde_json::from_value(payload_content)
                else {
                    // The Only reason this should fail is that the type field is missing, which is fine.
                    continue;
                };
                match type_name.as_str() {
                    "journal-mood" => {
                        let mood_value = payload.as_f64();
                        let Some(mood_value) = mood_value else {
                            tracing::error!(%conversation_id, %message_id, "invalid mood value");
                            continue;
                        };
                        let mood_value_f32 = mood_value.to_f32();
                        if let Some(mood_value_f32) = mood_value_f32
                            && mood_value_f32.is_finite()
                        {
                            mood = Some(mood_value_f32);
                        }
                        tracing::error!(%conversation_id, %message_id, mood_value, "mood value out of range");
                    }
                    "journal-content" => {
                        let content_string = match display_type {
                            None => {
                                let serde_json::Value::String(content_string) = payload else {
                                    tracing::error!(%conversation_id, %message_id, "invalid content value");
                                    continue;
                                };
                                content_string
                            }
                            #[allow(clippy::single_match_else)]
                            Some(display_type) => match display_type.as_str() {
                                "duration" => {
                                    let serde_json::Value::Number(content_seconds) = payload else {
                                        tracing::error!(%conversation_id, %message_id, "invalid content value");
                                        continue;
                                    };
                                    let Some(seconds) = content_seconds.as_u64() else {
                                        tracing::error!(%conversation_id, %message_id, "invalid content value");
                                        continue;
                                    };
                                    seconds_to_string(seconds)
                                }
                                _ => {
                                    tracing::error!(%conversation_id, %message_id, "unknown display type");
                                    continue;
                                }
                            },
                        };

                        content.push(JournalContentData {
                            prompt: last_prompt.take(),
                            content: content_string,
                            created_at,
                            updated_at,
                        });
                    }
                    "journal-title" => {
                        let serde_json::Value::String(title_value) = payload else {
                            tracing::error!(%conversation_id, %message_id, "invalid title value");
                            continue;
                        };
                        title = Some(title_value.trim().to_string());
                    }
                    "journal-focus" => {
                        let serde_json::Value::Array(focus_ids) = payload else {
                            tracing::error!(%conversation_id, %message_id, "invalid focuses value");
                            continue;
                        };
                        let mut temp = HashSet::new();
                        for focus_id in focus_ids {
                            let serde_json::Value::String(focus_id) = focus_id else {
                                tracing::error!(%conversation_id, %message_id, "invalid focus value");
                                continue;
                            };
                            let focus_id = match Uuid::parse_str(&focus_id) {
                                Ok(focus_id) => focus_id,
                                Err(error) => {
                                    tracing::error!(%conversation_id, %message_id, %error, "invalid focus id");
                                    continue;
                                }
                            };
                            temp.insert(focus_id);
                        }
                        focus = temp;
                    }
                    _ => {
                        // unknown type => ignore
                    }
                }
            }
            "journalcontentinput" => {
                let JournalPrompt { prompt } = match serde_json::from_value(payload_content) {
                    Ok(prompt) => prompt,
                    Err(error) => {
                        // We ignore the error because it is not recoverable
                        tracing::error!(error = &error as &dyn Error, %conversation_id, message_id = %message.id, "failed to parse message payload");
                        continue;
                    }
                };
                last_prompt = Some(prompt);
            }
            _ => {
                // unknown content type => ignore
            }
        }
    }

    JournalData {
        title,
        content,
        mood,
        focus,
    }
}

pub(crate) async fn create_session_journal_entry<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    session_title: &str,
    conversation_id: Uuid,
) -> Result<Option<MetaJournalEntryWithMetaContent>, MessagingError> {
    tracing::debug!(%conversation_id, "creating journal entry");
    let mut db = AsyncDatabase::sea_orm(conn);
    let conversation = csml_engine::future::get_conversation(&mut db, conversation_id).await?;
    let messages =
        csml_engine::future::db_connectors::messages::get_conversation_messages(&mut db, conversation_id).await?;

    let journal_data = create_journal_entities(conversation_id, messages);

    let user_id = conversation.client.user_id;
    let user_uuid = Uuid::parse_str(&user_id)?;

    let title = Some(journal_data.title.unwrap_or_else(|| session_title.to_string())).into_active_value();

    let res = conn
        .transaction(|txn| {
            Box::pin(async move {
                let entry = ActiveJournalEntry {
                    id: Uuid::new_v4().into_active_value(),
                    user_id: user_uuid.into_active_value(),
                    title,
                    mood: journal_data.mood.into_active_value(),
                    ..Default::default()
                };
                let created_entry: JournalEntryModel = JournalEntry::insert(entry).exec_with_returning(txn).await?;
                let contents = journal_data.content.into_iter().map(|content| {
                    let content = ActiveJournalContent {
                        id: Uuid::new_v4().into_active_value(),
                        journal_entry_id: created_entry.id.into_active_value(),
                        title: content.prompt.into_active_value(),
                        content: content.content.into_active_value(),
                        created_at: content.created_at.into_active_value(),
                        updated_at: content.updated_at.into_active_value(),
                    };

                    async move {
                        let created_content = JournalContent::insert(content).exec_with_returning(txn).await?;
                        Result::<_, DbErr>::Ok(MetaContent {
                            id: created_content.id,
                            journal_entry_id: created_content.journal_entry_id,
                            created_at: created_content.created_at,
                            updated_at: created_content.updated_at,
                        })
                    }
                });

                let focuses = journal_data.focus.into_iter().map(|focus| async move {
                    let focus_model = tag::Query::get_user_focus(txn, user_uuid, focus).await?;
                    let Some(focus_model) = focus_model else {
                        tracing::error!(%conversation_id, %focus, "focus not found");
                        return Ok(None);
                    };
                    let focus = ActiveJournalEntryTag {
                        journal_entry_id: created_entry.id.into_active_value(),
                        tag_id: focus_model.id.into_active_value(),
                    };
                    JournalEntryTag::insert(focus).exec(txn).await?;
                    Result::<_, DbErr>::Ok(Some(Tag::from_db_model(focus_model)))
                });
                let contents = try_join_all(contents);
                let focuses = try_join_all(focuses);
                let (contents, focuses) = try_join(contents, focuses).await?;
                Result::<_, DbErr>::Ok((created_entry, contents, focuses.into_iter().flatten().collect()))
            })
        })
        .await;

    let (created_entry, content, focus) = res.flatten_res()?;

    Ok(Some(MetaJournalEntryWithMetaContent {
        id: created_entry.id,
        user_id: user_uuid,
        title: created_entry.title,
        created_at: created_entry.created_at.fixed_offset(),
        updated_at: created_entry.updated_at.fixed_offset(),
        content,
        focus,
        mood: created_entry.mood,
        prompts: vec![],
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_duration() {
        assert_eq!(seconds_to_string(60).as_str(), "1 Minute");
        assert_eq!(seconds_to_string(365).as_str(), "6 Minuten");
        assert_eq!(seconds_to_string(3665).as_str(), "1 Stunde 1 Minute");
        assert_eq!(seconds_to_string(3720).as_str(), "1 Stunde 2 Minuten");
    }
}
