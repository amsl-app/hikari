use super::{LlmExecutionError, LlmStepContent};
use crate::builder::slot::paths::SlotPath;
use crate::builder::steps::{Condition, InjectionTrait, SlotsTrait, Template};
use crate::execution::steps::{LlmStepResponse, LlmStepTrait};
use crate::execution::utils::get_slots;
use async_stream::stream;
use futures_core::future::BoxFuture;
use futures_util::FutureExt;
use hikari_config::module::llm_agent::LlmService;
use hikari_core::llm_config::LlmConfig;
use hikari_core::openai::streaming::MessageStream;
use hikari_core::openai::{Content, Message};
use hikari_model::llm::state::{LlmConversationState, LlmStepStatus};
use sea_orm::DatabaseConnection;
use tokio::time::{Duration, sleep};
use uuid::Uuid;

#[derive(Clone)]
pub struct TextMessage {
    id: String,
    message: Template,
    hold: bool,
    conditions: Vec<Condition>,
    status: LlmStepStatus,
}

impl TextMessage {
    #[must_use]
    pub fn new(id: String, message: Template, hold: bool, conditions: Vec<Condition>) -> Self {
        Self {
            id,
            message,
            hold,
            conditions,
            status: LlmStepStatus::NotStarted,
        }
    }
}

impl LlmStepTrait for TextMessage {
    fn call<'a>(
        &'a mut self,
        _config: &'a LlmConfig,
        conversation_id: &'a Uuid,
        user_id: &'a Uuid,
        module_id: &'a str,
        session_id: &'a str,
        _llm_service: LlmService,
        conn: DatabaseConnection,
    ) -> BoxFuture<'a, Result<LlmStepResponse, LlmExecutionError>> {
        async move {
            let slots: Vec<SlotPath> = self.message.injection_slots();
            let slots = get_slots(&conn, conversation_id, user_id, module_id, session_id, slots).await?;
            let message = self.message.inject(&slots).to_string();

            let stream = stream! {
                let chars: Vec<char> = message.chars().collect();
                for chunk in chars.chunks(16) {
                    let chunk_str: String = chunk.iter().collect();
                    yield Ok(Message {
                        content: Content::Text(chunk_str), tokens: None
                    });
                    // Simulate delay (adjust as needed)
                    sleep(Duration::from_millis(50)).await;
                }
            };
            let stream = Box::pin(stream);
            let content = LlmStepContent::Message {
                message: stream,
                store: None,
            };
            Ok(LlmStepResponse::new(content, None))
        }
        .boxed()
    }

    fn add_previous_response(&mut self, _response: String) {
        // Nothing will happen here; Response is fixed in the step
    }

    fn remove_previous_response(&mut self) {
        // Nothing will happen here; Function gets called at the beginning of the step
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
        &self.conditions
    }

    fn id(&self) -> &str {
        &self.id
    }
}
