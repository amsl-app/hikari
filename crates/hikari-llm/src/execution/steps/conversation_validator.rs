use super::LlmStepContent;
use crate::builder::slot::SaveTarget;
use crate::builder::slot::paths::{Destination, SlotPath};
use crate::builder::steps::Condition;
use crate::builder::steps::validator::ValidationType;
use crate::execution::core::LlmCore;
use crate::execution::error::LlmExecutionError;
use crate::execution::steps::{LlmStepResponse, LlmStepTrait};
use futures_core::future::BoxFuture;
use futures_util::FutureExt;
use hikari_config::module::llm_agent::LlmService;
use hikari_core::llm_config::LlmConfig;
use hikari_core::openai::{Content, Message};
use hikari_model::llm::state::{LlmConversationState, LlmStepStatus};
use sea_orm::DatabaseConnection;
use serde_yml::Value;
use std::collections::HashMap;
use uuid::Uuid;

pub type NextStep = Option<String>;

#[derive(Clone)]
pub struct ConversationValidator {
    id: String,
    core: LlmCore,
    goto_on_success: NextStep,
    goto_on_fail: NextStep,
    conditions: Vec<Condition>,
    status: LlmStepStatus,
    previous_response: Option<String>,
    validation_type: ValidationType,
}

impl ConversationValidator {
    #[must_use]
    pub fn new(
        id: String,
        core: LlmCore,
        goto_on_success: NextStep,
        goto_on_fail: NextStep,
        conditions: Vec<Condition>,
        validation_type: ValidationType,
    ) -> Self {
        Self {
            id,
            core,
            goto_on_success,
            goto_on_fail,
            conditions,
            status: LlmStepStatus::NotStarted,
            previous_response: None,
            validation_type,
        }
    }
}

impl LlmStepTrait for ConversationValidator {
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

                let values: HashMap<String, Value> = serde_json::from_value(arguments)?;

                let slots = values
                    .iter()
                    .map(|(k, v)| {
                        (
                            SaveTarget::Slot(SlotPath::new(k.to_owned(), Destination::default())),
                            v.clone(),
                        )
                    })
                    .collect();

                let decisions: Vec<(String, bool)> = values
                    .iter()
                    .filter_map(|(k, v)| Value::as_bool(v).map(|b| (k.to_owned(), b)))
                    .collect();

                if decisions.len() != values.len() / 2 {
                    return Err(LlmExecutionError::Unexpected(
                        "Expected a boolean value for every validation goal".to_owned(),
                    ));
                }

                let success = match self.validation_type {
                    ValidationType::All => decisions.iter().all(|(_, v)| *v),
                    ValidationType::Any => decisions.iter().any(|(_, v)| *v),
                };

                let goto = if success {
                    self.goto_on_success.clone()
                } else {
                    self.goto_on_fail.clone()
                };

                let content = LlmStepContent::StepValue {
                    values: slots,
                    next_step: goto,
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
