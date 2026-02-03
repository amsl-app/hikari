use super::{LlmStepResponse, LlmStepTrait};
use crate::builder::slot::SaveTarget;
use crate::builder::slot::paths::SlotPath;
use crate::builder::steps::api::{ApiHeader, ApiMethod};
use crate::builder::steps::{InjectionTrait, Template};
use crate::execution::error::APIExecutionError;
use crate::execution::steps::LlmStepContent;
use crate::execution::utils::get_slots;
use crate::{builder::steps::Condition, execution::error::LlmExecutionError};
use async_stream::try_stream;
use futures_core::future::BoxFuture;
use futures_util::{FutureExt, StreamExt};
use hikari_config::module::llm_agent::LlmService;
use hikari_core::llm_config::LlmConfig;
use hikari_core::openai::streaming::MessageStream;
use hikari_core::openai::{Content, Message};
use hikari_model::llm::state::{LlmConversationState, LlmStepStatus};
use hikari_utils::values::{JsonToYaml, QueryJson, ValueDecoder, YamlToJson};
use reqwest_sse::EventSource;
use sea_orm::DatabaseConnection;
use uuid::Uuid;

#[derive(Clone)]
pub struct SseCall {
    id: String,
    slots: Vec<SlotPath>,
    url: String,
    method: ApiMethod,
    headers: Vec<ApiHeader>,
    body: Option<Template>,
    response_path: Option<String>, // Path of the json response to extract data from for the slot
    store: Option<SaveTarget>,
    conditions: Vec<Condition>,
    status: LlmStepStatus,
}

impl SseCall {
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        slots: Vec<SlotPath>,
        url: String,
        method: ApiMethod,
        headers: Vec<ApiHeader>,
        body: Option<Template>,
        response_path: Option<String>,
        store: Option<SaveTarget>,
        conditions: Vec<Condition>,
    ) -> Self {
        Self {
            id,
            slots,
            url,
            method,
            headers,
            body,
            response_path,
            store,
            conditions,
            status: LlmStepStatus::NotStarted,
        }
    }
}

impl LlmStepTrait for SseCall {
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
            let values = get_slots(
                &conn,
                conversation_id,
                user_id,
                module_id,
                session_id,
                self.slots.clone(),
            )
            .await?;

            let headers: Vec<ApiHeader> = self.headers.iter().map(|header| header.inject(&values)).collect();

            let body = self.body.as_ref().map(|body| body.inject(&values));

            let client = reqwest::Client::new();

            let request = match self.method {
                ApiMethod::GET => client.get(&self.url),
                ApiMethod::POST => client.post(&self.url),
                ApiMethod::PUT => client.put(&self.url),
                ApiMethod::DELETE => client.delete(&self.url),
            };

            let request = headers
                .into_iter()
                .fold(request, |req, header| req.header(header.key, header.value.to_string()));

            let request = if let Some(body) = body {
                let body_string = body.as_ref().to_json_string()?;
                request.body(body_string)
            } else {
                request
            };

            let mut events = request
                .send()
                .await
                .map_err(APIExecutionError::ReqwestError)?
                .events()
                .await
                .map_err(APIExecutionError::ReqwestSseEventSourceError)?;

            let response_path = self.response_path.clone();
            let result = try_stream! {
                while let Some(Ok(event)) = events.next().await {
                    let data = &event.data;

                    let content: String = if let Some(path) = &response_path {
                        let json_value: serde_json::Value = serde_json::from_str(data)?;
                        json_value.query(path).map(|v| v.to_yaml())??.encode()
                    } else {
                        data.to_owned()
                    };

                    yield Message {
                        content: Content::Text(content), tokens: None
                    };
                }
            }
            .boxed();

            Ok(LlmStepResponse::new(
                LlmStepContent::Message {
                    message: MessageStream::new(result),
                    store: self.store.clone(),
                },
                None,
            ))
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
