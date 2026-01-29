use crate::builder::slot::SaveTarget;
use crate::builder::slot::paths::SlotPath;
use crate::builder::steps::Condition;
use crate::execution::core::LlmCore;
use crate::execution::error::LlmExecutionError;
use crate::execution::steps::conversation_validator::NextStep;
use crate::execution::steps::{LlmStepContent, LlmStepResponse, LlmStepTrait};
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

#[derive(Clone)]
pub struct ValueExtractor {
    id: String,
    core: LlmCore,
    slots: Vec<SaveTarget>,
    goto_on_success: NextStep,
    goto_on_fail: NextStep,
    conditions: Vec<Condition>,
    status: LlmStepStatus,
    previous_response: Option<String>,
}

impl ValueExtractor {
    #[must_use]
    pub fn new(
        id: String,
        core: LlmCore,
        slots: Vec<SaveTarget>,
        goto_on_success: NextStep,
        goto_on_fail: NextStep,
        conditions: Vec<Condition>,
    ) -> Self {
        Self {
            id,
            core,
            slots,
            goto_on_success,
            goto_on_fail,
            conditions,
            status: LlmStepStatus::NotStarted,
            previous_response: None,
        }
    }
}

impl LlmStepTrait for ValueExtractor {
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
            let mut step_values = HashMap::new();

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

                tracing::info!("ValueExtractor response: {:?}", values);

                let mut success = true;

                for target in &self.slots {
                    let value = match target {
                        SaveTarget::Slot(SlotPath { name, .. }) => {
                            let value = values.get(name).map(serde_yml::to_value).transpose()?;

                            match value {
                                Some(Value::String(str_value))
                                    if str_value.trim().is_empty()
                                        || str_value.eq_ignore_ascii_case("null")
                                        || str_value.eq_ignore_ascii_case("none") =>
                                {
                                    None
                                }
                                _ => value,
                            }
                        }
                    };

                    if let Some(value) = value {
                        if value.is_null() {
                            success = false;
                        } else {
                            step_values.insert(target.clone(), value);
                        }
                    } else {
                        success = false;
                    }
                }

                let content = if success {
                    LlmStepContent::StepValue {
                        values: step_values,
                        next_step: self.goto_on_success.clone(),
                    }
                } else {
                    LlmStepContent::StepValue {
                        values: step_values,
                        next_step: self.goto_on_fail.clone(),
                    }
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
