use super::{LlmStepContent, LlmStepResponse, LlmStepTrait};
use crate::execution::steps::conversation_validator::NextStep;
use crate::{builder::steps::Condition, execution::error::LlmExecutionError};
use futures_core::future::BoxFuture;
use futures_util::FutureExt;
use hikari_config::module::llm_agent::LlmService;
use hikari_core::llm_config::LlmConfig;
use hikari_model::llm::state::{LlmConversationState, LlmStepStatus};
use sea_orm::DatabaseConnection;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone)]
pub struct GoTo {
    id: String,
    next_step: NextStep,
    conditions: Vec<Condition>,
    status: LlmStepStatus,
}

impl GoTo {
    #[must_use]
    pub fn new(id: String, next_step: NextStep, conditions: Vec<Condition>) -> Self {
        Self {
            id,
            next_step,
            conditions,
            status: LlmStepStatus::NotStarted,
        }
    }
}

impl LlmStepTrait for GoTo {
    fn call<'a>(
        &'a mut self,
        _config: &'a LlmConfig,
        _conversation_id: &'a Uuid,
        _user_id: &'a Uuid,
        _module_id: &'a str,
        _session_id: &'a str,
        _llm_service: LlmService,
        _conn: DatabaseConnection,
    ) -> BoxFuture<'a, Result<LlmStepResponse, LlmExecutionError>> {
        async move {
            Ok(LlmStepResponse::new(
                LlmStepContent::StepValue {
                    values: HashMap::new(),
                    next_step: self.next_step.clone(),
                },
                None,
            ))
        }
        .boxed()
    }

    fn add_previous_response(&mut self, _response: String) {
        tracing::error!(
            "Adding previous response to goto should not happen, since this step does not produce a response."
        );
    }

    fn remove_previous_response(&mut self) {
        // Nothing will happen here; Function gets called at the beginning of the step
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
        &self.conditions
    }

    fn id(&self) -> &str {
        &self.id
    }
}
