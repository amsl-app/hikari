use futures_core::future::BoxFuture;
use futures_util::FutureExt;
use futures_util::future::try_join_all;
use hikari_config::module::llm_agent::LlmService;
use hikari_core::llm_config::LlmConfig;
use hikari_model::llm::state::{LlmConversationState, LlmStepStatus};
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use crate::builder::steps::Condition;
use crate::execution::error::LlmExecutionError;
use crate::execution::steps::{LlmStepContent, LlmStepResponse, LlmStepTrait};

use super::LlmStep;

#[derive(Clone)]
pub struct CombinedStep {
    id: String,
    steps: Vec<LlmStep>,
    conditions: Vec<Condition>,
}

impl CombinedStep {
    #[must_use]
    pub fn new(id: String, steps: Vec<LlmStep>, conditions: Vec<Condition>) -> Self {
        Self { id, steps, conditions }
    }
}

impl LlmStepTrait for CombinedStep {
    fn call<'a>(
        &'a mut self,
        config: &'a LlmConfig,
        conversatoin_id: &'a Uuid,
        user_id: &'a Uuid,
        module_id: &'a str,
        session_id: &'a str,
        llm_service: LlmService,
        conn: DatabaseConnection,
    ) -> BoxFuture<'a, Result<LlmStepResponse, LlmExecutionError>> {
        async move {
            let executions = self.steps.iter_mut().map(|step| {
                step.execute(
                    config,
                    conversatoin_id,
                    user_id,
                    module_id,
                    session_id,
                    llm_service.clone(),
                    conn.clone(),
                )
            });
            tracing::trace!(id = ?self.id, "Executing combined step");
            let responses = try_join_all(executions).await?;
            tracing::trace!(id = ?self.id, "Combined step executed successfully");
            let content = LlmStepContent::Combined(responses);
            Ok(LlmStepResponse::new(content, None))
        }
        .boxed()
    }

    fn add_previous_response(&mut self, response: String) {
        for step in &mut self.steps {
            step.add_previous_response(response.clone());
        }
    }

    fn remove_previous_response(&mut self) {
        for step in &mut self.steps {
            step.remove_previous_response();
        }
    }

    fn set_status(&mut self, status: LlmStepStatus) -> LlmConversationState {
        for step in &mut self.steps {
            step.set_status(status);
        }
        self.state()
    }

    fn finish(&mut self) -> LlmConversationState {
        for step in &mut self.steps {
            step.finish();
        }
        self.state()
    }

    fn status(&self) -> LlmStepStatus {
        // We want to finde the most important status of all the steps
        // Error > NotStarted > Running > WaitingForInput > Completed
        let status: Vec<LlmStepStatus> = self.steps.iter().map(super::LlmStepTrait::status).collect();

        if status.contains(&LlmStepStatus::Error) {
            LlmStepStatus::Error
        } else if status.contains(&LlmStepStatus::NotStarted) {
            LlmStepStatus::NotStarted
        } else if status.contains(&LlmStepStatus::Running) {
            LlmStepStatus::Running
        } else if status.contains(&LlmStepStatus::WaitingForInput) {
            LlmStepStatus::WaitingForInput
        } else {
            LlmStepStatus::Completed
        }
    }

    fn conditions(&self) -> &[Condition] {
        self.conditions.as_slice()
    }

    fn id(&self) -> &str {
        &self.id
    }
}
