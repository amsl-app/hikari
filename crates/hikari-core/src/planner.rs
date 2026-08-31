use std::fmt::Write;
use std::time::Duration;

use chrono::NaiveDate;
use hikari_model::planner::NewPlannerEntry;
use schemars::JsonSchema;
use sea_orm::{DatabaseConnection, prelude::Uuid};
use serde::{Deserialize, Serialize};

use tracing::instrument;

use crate::{
    llm_config::LlmConfig,
    openai::{CallConfig, error::OpenAiError, openai_single_tool_call},
    planner::error::PlannerAssistantError,
    usage::add_usage,
};
use async_openai::types::chat::{
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
};

pub mod error;

#[derive(Serialize, Deserialize, JsonSchema)]
#[schemars(title = "PlannerEntries", description = "Planner entries parsed from user input")]
struct PlannerEntriesResponse {
    entries: Vec<PlannerEntryResponse>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
struct PlannerEntryResponse {
    /// Short title for the task or event
    title: String,
    /// Date in ISO 8601 format (YYYY-MM-DD)
    date: String,
    /// Priority: 1 = low, 2 = medium, 3 = high
    priority: i32,
    /// ID (UUID) of the matching milestone from the provided list, or null if none fits
    milestone_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PlannerAssistantExistingEntry {
    pub date: NaiveDate,
    pub title: String,
}

#[derive(Debug, Clone)]
pub struct PlannerAssistantMilestone {
    pub id: Uuid,
    pub title: String,
    pub date: NaiveDate,
}

#[allow(clippy::too_many_arguments)]
#[instrument(skip(llm_config, conn), err)]
pub async fn planner_assistant(
    user_id: &Uuid,
    text: String,
    today: NaiveDate,
    milestones: Vec<PlannerAssistantMilestone>,
    existing_entries: Vec<PlannerAssistantExistingEntry>,
    llm_config: &LlmConfig,
    conn: &DatabaseConnection,
) -> Result<Vec<NewPlannerEntry>, PlannerAssistantError> {
    let system_content = build_system_prompt(today, &milestones, &existing_entries);

    let messages: Vec<ChatCompletionRequestMessage> = vec![
        ChatCompletionRequestSystemMessageArgs::default()
            .content(system_content)
            .build()
            .map_err(OpenAiError::from)?
            .into(),
        ChatCompletionRequestUserMessageArgs::default()
            .content(text)
            .build()
            .map_err(OpenAiError::from)?
            .into(),
    ];

    tracing::info!("sending {} messages to openAI for planner assistant", messages.len());

    let openai_config = llm_config.get_planner_openai_config();
    let model = llm_config.get_planner_model();

    let (res, tokens) = openai_single_tool_call::<PlannerEntriesResponse>(
        CallConfig::builder()
            .iteration_timeout(Duration::from_secs(30))
            .total_timeout(Duration::from_mins(2))
            .build(),
        openai_config,
        None,
        None,
        model,
        messages,
    )
    .await?;

    if let Some(usage) = tokens {
        add_usage(conn, user_id, usage, "planner_assistant").await?;
    }

    res.entries
        .into_iter()
        .filter(|e| !e.title.trim().is_empty())
        .map(|e| {
            let PlannerEntryResponse {
                title,
                date,
                priority,
                milestone_id,
            } = e;
            let parsed_date =
                NaiveDate::parse_from_str(&date, "%Y-%m-%d").map_err(|_| PlannerAssistantError::InvalidDate(date))?;
            let milestone_id = milestone_id.and_then(|id| Uuid::parse_str(&id).ok());
            Ok(NewPlannerEntry {
                date: parsed_date,
                title: title.trim().to_owned(),
                priority: priority.clamp(1, 3),
                milestone_id,
            })
        })
        .collect()
}

fn build_system_prompt(
    today: NaiveDate,
    milestones: &[PlannerAssistantMilestone],
    existing_entries: &[PlannerAssistantExistingEntry],
) -> String {
    let exisiting_entires_limit = 10;

    let mut content = format!(
        "Du bist ein Planungsassistent, der Aufgaben und Termine aus Freitext extrahiert.\n\
         Heutiges Datum: {today}.\n\n"
    );

    if !milestones.is_empty() {
        content.push_str("Verfügbare Meilensteine (verwende die exakte ID bei der Zuordnung):\n");
        for m in milestones {
            writeln!(content, "- \"{}\": {} (fällig am {})", m.id, m.title, m.date)
                .expect("Writing to a String can't fail");
        }
        content.push('\n');
    }

    if !existing_entries.is_empty() {
        content.push_str("Bereits geplante Einträge (als Kontext, um Duplikate zu vermeiden):\n");
        for e in existing_entries.iter().take(exisiting_entires_limit) {
            writeln!(content, "- {}: {}", e.date, e.title).expect("Writing to a String can't fail");
        }
        content.push('\n');
    }

    content.push_str(
        "Extrahiere alle einzelnen Aufgaben oder Termine aus dem Text des Nutzers. Für jeden Eintrag:\n\
         - Lege einen kurzen, klaren Titel fest\n\
         - Bestimme das Datum im ISO-8601-Format (YYYY-MM-DD); berechne absolute Daten für relative Ausdrücke wie \"morgen\" oder \"nächsten Montag\" basierend auf dem heutigen Datum\n\
         - Lege die Priorität fest: 1 = niedrig, 2 = mittel, 3 = hoch (Standard 2, falls nicht angegeben)\n\
         - Setze milestone_id nur, wenn die Aufgabe eindeutig zu einem der angegebenen Meilensteine passt\n\
         - Erstelle neue Einträge immer auf Deutsch
         Rufe die Funktion `PlannerEntries` mit allen extrahierten Einträgen auf.",
    );

    content
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_lists_milestones() {
        let milestones = vec![PlannerAssistantMilestone {
            id: Uuid::nil(),
            title: "Midterm".to_owned(),
            date: NaiveDate::from_ymd_opt(2026, 8, 1).unwrap(),
        }];
        let prompt = build_system_prompt(NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(), &milestones, &[]);
        assert!(prompt.contains("Verfügbare Meilensteine"));
        assert!(prompt.contains("Midterm"));
        assert!(prompt.contains("2026-08-01"));
    }
}
