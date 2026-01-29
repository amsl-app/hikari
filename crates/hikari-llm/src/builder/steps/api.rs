use std::collections::HashMap;

use schemars::JsonSchema;
use serde::Deserialize;
use serde_yml::Value;

use super::{Condition, IntoLlmStep, ParentStep};
use crate::{
    builder::{
        error::LlmBuildingError,
        slot::{SaveTarget, SlotValuePair, paths::SlotPath},
        step_id_from_flow,
        steps::{Documents, Flow, InjectionTrait, SlotsTrait, Template},
    },
    execution::steps::{LlmStep, api_call::ApiCall},
};

#[derive(Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ApiHeader {
    pub key: String,
    pub value: Template,
}

impl SlotsTrait for ApiHeader {
    fn injection_slots(&self) -> Vec<crate::builder::slot::paths::SlotPath> {
        self.value.injection_slots()
    }
}

impl InjectionTrait for ApiHeader {
    fn inject(&self, values: &[SlotValuePair]) -> Self {
        ApiHeader {
            key: self.key.clone(),
            value: self.value.inject(values),
        }
    }
}

#[derive(Deserialize, Debug, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub enum ApiMethod {
    GET,
    POST,
    PUT,
    DELETE,
}
#[derive(Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ApiBuilder {
    pub url: String,
    pub method: ApiMethod,
    #[serde(default)]
    pub headers: Vec<ApiHeader>,
    #[serde(default)]
    pub body: Option<Template>,
    pub success: Flow,
    pub fail: Flow,
    #[serde(default)]
    pub response_path: Option<String>, // Path of the json response to extract data from for the slot
    pub target: SaveTarget,
}

impl SlotsTrait for ApiBuilder {
    fn injection_slots(&self) -> Vec<crate::builder::slot::paths::SlotPath> {
        let mut slots = self
            .body
            .as_ref()
            .map_or_else(Vec::new, super::SlotsTrait::injection_slots);
        slots.extend(self.headers.iter().flat_map(SlotsTrait::injection_slots));
        slots
    }
}

impl IntoLlmStep for ApiBuilder {
    fn into_llm_step(
        self,
        parent_steps: Vec<ParentStep>,
        mut conditions: Vec<Condition>,
        id: String,
        _constants: HashMap<String, Value>,
        _documents: Documents,
    ) -> Result<LlmStep, LlmBuildingError> {
        let slots: Vec<SlotPath> = self.injection_slots();

        let ApiBuilder {
            url,
            method,
            headers,
            body,
            response_path,
            target: store,
            success,
            fail,
        } = self;

        let goto_on_success = step_id_from_flow(success, &parent_steps);
        let goto_on_fail = step_id_from_flow(fail, &parent_steps);

        for step in parent_steps {
            conditions.extend(step.conditions);
        }

        Ok(LlmStep::ApiCall(ApiCall::new(
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
        )))
    }
}
