use super::LlmStepContent;
use crate::builder::slot::SaveTarget;
use crate::builder::slot::paths::{Destination, SlotPath};
use crate::builder::steps::Condition;
use crate::builder::steps::summarizer::UpdateType;
use crate::execution::core::LlmCore;
use crate::execution::error::LlmExecutionError;
use crate::execution::steps::{LlmStepResponse, LlmStepTrait};
use crate::execution::utils::get_conversation_slots;
use futures_core::future::BoxFuture;
use futures_util::FutureExt;
use hikari_config::module::llm_agent::LlmService;
use hikari_core::llm_config::LlmConfig;
use hikari_core::openai::{Content, Message};
use hikari_model::llm::state::{LlmConversationState, LlmStepStatus};
use hikari_utils::values::ValueDecoder;
use sea_orm::DatabaseConnection;
use serde_yml::Value;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone)]
pub struct ConversationSummarizer {
    id: String,
    core: LlmCore,
    update_type: UpdateType,
    conditions: Vec<Condition>,
    status: LlmStepStatus,
    previous_response: Option<String>,
}

impl ConversationSummarizer {
    #[must_use]
    pub fn new(id: String, core: LlmCore, update_type: UpdateType, conditions: Vec<Condition>) -> Self {
        Self {
            id,
            core,
            update_type,
            conditions,
            status: LlmStepStatus::NotStarted,
            previous_response: None,
        }
    }
}

impl LlmStepTrait for ConversationSummarizer {
    fn call<'a>(
        &'a mut self,
        config: &'a LlmConfig,
        conversation_id: &'a Uuid,
        user_id: &'a Uuid,
        module_id: &'a str,
        session_id: &'a str,
        llm_service: LlmService,
        conn: DatabaseConnection,
    ) -> BoxFuture<'a, Result<LlmStepResponse, LlmExecutionError>> {
        async move {
            let slots = get_conversation_slots(&conn, conversation_id, vec!["summary".to_owned()]).await?;

            let Message { content, tokens } = self
                .core
                .invoke(
                    config,
                    conversation_id,
                    user_id,
                    module_id,
                    session_id,
                    llm_service,
                    conn,
                    self.previous_response.take(),
                )
                .await?;

            if let Content::Tool(tool_calls) = content {
                let first = tool_calls
                    .into_iter()
                    .next()
                    .ok_or(LlmExecutionError::UnexpectedResponseFormat)?;
                let arguments = first.arguments;
                // FIXME: This should probably be checked
                let summary = arguments
                    .get("summary")
                    .expect("missing summary")
                    .as_str()
                    .unwrap_or("")
                    .to_string();

                let new_summary = if let UpdateType::Append = self.update_type {
                    let previous = slots.first().map(|s| s.value.encode());

                    let previous = previous.map_or(String::new(), |p| format!("{p}\n"));
                    format!("{previous}{summary}")
                } else {
                    summary
                };

                let target = SaveTarget::Slot(SlotPath::new("summary".to_owned(), Destination::default()));
                let mut slot: HashMap<SaveTarget, Value> = HashMap::new();
                slot.insert(target, Value::String(new_summary));
                let content = LlmStepContent::StepValue {
                    values: slot,
                    next_step: None,
                };
                Ok(LlmStepResponse::new(content, tokens))
            } else {
                Err(LlmExecutionError::UnexpectedResponseFormat)
            }
        }
        .boxed()
    }

    fn add_previous_response(&mut self, response: String) {
        self.previous_response = Some(response);
    }

    fn remove_previous_response(&mut self) {
        self.previous_response = None;
    }

    fn set_status(&mut self, status: LlmStepStatus) -> LlmConversationState {
        self.status = status;
        self.state()
    }

    fn finish(&mut self) -> LlmConversationState {
        self.set_status(LlmStepStatus::Completed);
        self.state()
    }

    fn status(&self) -> LlmStepStatus {
        self.status
    }

    fn conditions(&self) -> &[Condition] {
        self.conditions.as_slice()
    }

    fn id(&self) -> &str {
        &self.id
    }
}
