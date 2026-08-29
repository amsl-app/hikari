use super::{LlmStepResponse, LlmStepTrait};
use crate::builder::steps::media::MediaType;
use crate::builder::steps::{InjectionTrait, Template};
use crate::execution::steps::LlmStepContent;
use crate::{builder::steps::Condition, execution::error::LlmExecutionError};
use futures_core::future::BoxFuture;
use futures_util::FutureExt;
use hikari_config::module::llm_agent::LlmService;
use hikari_core::llm_config::LlmConfig;
use hikari_model::llm::state::{LlmConversationState, LlmStepStatus};

use sea_orm::DatabaseConnection;
use uuid::Uuid;

#[derive(Clone)]
pub struct MediaFetch {
    id: String,
    url: Template,
    r#type: MediaType,
    conditions: Vec<Condition>,
    status: LlmStepStatus,
}

impl MediaFetch {
    #[must_use]
    pub fn new(id: String, url: Template, r#type: MediaType, conditions: Vec<Condition>) -> Self {
        Self {
            id,
            url,
            r#type,
            conditions,
            status: LlmStepStatus::NotStarted,
        }
    }
}

impl LlmStepTrait for MediaFetch {
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
            let url = self
                .url
                .resolve(conversation_id, user_id, module_id, session_id, &conn)
                .await?;

            Ok(LlmStepResponse {
                content: LlmStepContent::Media {
                    url: url.to_string(),
                    r#type: self.r#type.clone(),
                },
                tokens: None,
            })
        }
        .boxed()
    }

    fn add_previous_response(&mut self, _response: String) {
        tracing::error!(
            "Adding previous response to api_call should not happen, since this step does not produce a response."
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
