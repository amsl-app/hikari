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
    let mut content = format!(
        "You are a planning assistant that extracts tasks and events from free text.\n\
         Today's date is {today}.\n\n"
    );

    if !milestones.is_empty() {
        content.push_str("Available milestones (use the exact ID when assigning):\n");
        for m in milestones {
            let _ = writeln!(content, "- \"{}\": {} (due {})", m.id, m.title, m.date);
        }
        content.push('\n');
    }

    if !existing_entries.is_empty() {
        content.push_str("Already planned entries (for context, avoid creating duplicates):\n");
        for e in existing_entries {
            let _ = writeln!(content, "- {}: {}", e.date, e.title);
        }
        content.push('\n');
    }

    content.push_str(
        "Extract all distinct tasks or events from the user's text. For each entry:\n\
         - Set a short, clear title\n\
         - Determine the date in ISO 8601 format (YYYY-MM-DD); calculate absolute dates for relative expressions like \"tomorrow\" or \"next Monday\" based on today's date\n\
         - Set priority: 1 = low, 2 = medium, 3 = high (default 2 if unspecified)\n\
         - Only set milestone_id if the task clearly relates to one of the provided milestones\n\
         Call the `PlannerEntries` function with all extracted entries.",
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
        assert!(prompt.contains("Available milestones"));
        assert!(prompt.contains("Midterm"));
        assert!(prompt.contains("2026-08-01"));
    }
}
