use super::LlmStepContent;
use crate::builder::slot::SaveTarget;
use crate::builder::steps::Condition;
use crate::execution::core::LlmCore;
use crate::execution::error::LlmExecutionError;
use crate::execution::steps::{LlmStepResponse, LlmStepTrait};
use futures_core::future::BoxFuture;
use futures_util::FutureExt;
use hikari_config::module::llm_agent::LlmService;
use hikari_core::llm_config::LlmConfig;
use hikari_model::llm::state::{LlmConversationState, LlmStepStatus};
use sea_orm::DatabaseConnection;
use uuid::Uuid;

#[derive(Clone)]
pub struct MessageGenerator {
    id: String,
    core: LlmCore,
    hold: bool,
    conditions: Vec<Condition>,
    status: LlmStepStatus,
    previous_response: Option<String>,
    store: Option<SaveTarget>,
}

impl MessageGenerator {
    #[must_use]
    pub fn new(id: String, core: LlmCore, hold: bool, conditions: Vec<Condition>, store: Option<SaveTarget>) -> Self {
        Self {
            id,
            core,
            hold,
            conditions,
            status: LlmStepStatus::NotStarted,
            previous_response: None,
            store,
        }
    }
}

impl LlmStepTrait for MessageGenerator {
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
            let message = self
                .core
                .stream(
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
            let content = LlmStepContent::Message {
                message,
                store: self.store.clone(),
            };
            Ok(LlmStepResponse::new(content, None))
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
        if self.hold {
            self.set_status(LlmStepStatus::WaitingForInput);
        } else {
            self.set_status(LlmStepStatus::Completed);
        }
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
