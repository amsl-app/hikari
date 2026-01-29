use std::collections::HashMap;

use super::{LlmStepResponse, LlmStepTrait};
use crate::builder::slot::SaveTarget;
use crate::builder::slot::paths::SlotPath;
use crate::builder::steps::api::{ApiHeader, ApiMethod};
use crate::builder::steps::{InjectionTrait, Template};
use crate::execution::error::APIExecutionError;
use crate::execution::steps::LlmStepContent;
use crate::execution::steps::conversation_validator::NextStep;
use crate::execution::utils::get_slots;
use crate::{builder::steps::Condition, execution::error::LlmExecutionError};
use futures_core::future::BoxFuture;
use futures_util::FutureExt;
use hikari_config::module::llm_agent::LlmService;
use hikari_core::llm_config::LlmConfig;
use hikari_model::llm::state::{LlmConversationState, LlmStepStatus};
use hikari_utils::values::{JsonToYaml, QueryJson, YamlToJson};
use sea_orm::DatabaseConnection;
use serde_yml::Value;
use uuid::Uuid;

#[derive(Clone)]
pub struct ApiCall {
    id: String,
    slots: Vec<SlotPath>,
    url: String,
    method: ApiMethod,
    headers: Vec<ApiHeader>,
    body: Option<Template>,
    response_path: Option<String>, // Path of the json response to extract data from for the slot
    store: SaveTarget,
    goto_on_success: NextStep,
    goto_on_fail: NextStep,
    conditions: Vec<Condition>,
    status: LlmStepStatus,
}

impl ApiCall {
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
        store: SaveTarget,
        goto_on_success: NextStep,
        goto_on_fail: NextStep,
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
            goto_on_success,
            goto_on_fail,
            conditions,
            status: LlmStepStatus::NotStarted,
        }
    }
}

impl LlmStepTrait for ApiCall {
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

            let response = request.send().await.map_err(APIExecutionError::ReqwestError)?;

            let success = response.status().is_success();

            let json_value: serde_json::Value = response
                .json()
                .await
                .map_err(|e| APIExecutionError::InvalidResponseFormat(e.to_string()))?;
            tracing::debug!(%json_value, "API response");

            let content: Value = if let Some(path) = &self.response_path {
                json_value.query(path).map(|v| v.to_yaml())??
            } else {
                json_value.to_yaml()?
            };

            let values = HashMap::from([(self.store.clone(), content)]);

            let content = if success {
                LlmStepContent::StepValue {
                    values,
                    next_step: self.goto_on_success.clone(),
                }
            } else {
                LlmStepContent::StepValue {
                    values,
                    next_step: self.goto_on_fail.clone(),
                }
            };

            Ok(LlmStepResponse::new(content, None))
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
