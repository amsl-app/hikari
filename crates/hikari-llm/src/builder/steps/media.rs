use std::collections::HashMap;

use schemars::JsonSchema;
use serde::Deserialize;
use yaml_serde::Value;

use super::{Condition, IntoLlmStep, ParentStep};
use crate::{
    builder::{
        error::LlmBuildingError,
        steps::{Documents, Template},
    },
    execution::steps::{LlmStep, media_fetch::MediaFetch},
};

#[derive(Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum MediaType {
    Image,
    Video,
}

#[derive(Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct MediaBuilder {
    pub url: Template,
    pub r#type: MediaType,
}

impl IntoLlmStep for MediaBuilder {
    fn into_llm_step(
        self,
        parent_steps: Vec<ParentStep>,
        mut conditions: Vec<Condition>,
        id: String,
        _constants: HashMap<String, Value>,
        _documents: Documents,
    ) -> Result<LlmStep, LlmBuildingError> {
        let MediaBuilder { url, r#type } = self;

        for step in parent_steps {
            conditions.extend(step.conditions);
        }

        Ok(LlmStep::Media(MediaFetch::new(id, url, r#type, conditions)))
    }
}
