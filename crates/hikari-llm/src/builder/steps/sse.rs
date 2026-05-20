use std::collections::HashMap;

use schemars::JsonSchema;
use serde::Deserialize;
use yaml_serde::Value;

use super::{Condition, IntoLlmStep, ParentStep};
use crate::{
    builder::{
        error::LlmBuildingError,
        slot::SaveTarget,
        steps::{
            Documents, Template,
            api::{ApiHeader, ApiMethod},
        },
    },
    execution::steps::{LlmStep, sse_call::SseCall},
};

#[derive(Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SseBuilder {
    pub url: String,
    pub method: ApiMethod,
    #[serde(default)]
    pub headers: Vec<ApiHeader>,
    #[serde(default)]
    pub body: Option<Template>,
    #[serde(default)]
    pub response_path: Option<String>, // Path of the json response to extract data from for the slot
    #[serde(default)]
    pub store: Option<SaveTarget>,
}

impl IntoLlmStep for SseBuilder {
    fn into_llm_step(
        self,
        parent_steps: Vec<ParentStep>,
        mut conditions: Vec<Condition>,
        id: String,
        _constants: HashMap<String, Value>,
        _documents: Documents,
    ) -> Result<LlmStep, LlmBuildingError> {
        let SseBuilder {
            url,
            method,
            headers,
            body,
            response_path,
            store,
        } = self;

        for step in parent_steps {
            conditions.extend(step.conditions);
        }

        Ok(LlmStep::SseCall(SseCall::new(
            id,
            url,
            method,
            headers,
            body,
            response_path,
            store,
            conditions,
        )))
    }
}
