use super::{Condition, IntoLlmStep, ParentStep};
use crate::builder::error::LlmBuildingError;
use crate::builder::steps::{Documents, Template};
use crate::execution::steps::LlmStep;
use crate::execution::steps::text_message::TextMessage;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_yml::Value;
use std::collections::HashMap;

#[derive(Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct MessageBuilder {
    pub message: Template,
    /// # Whether to hold the conversation after this message
    /// Hold means the user can send a response before continuing
    #[serde(default)]
    pub hold: bool,
}

impl IntoLlmStep for MessageBuilder {
    fn into_llm_step(
        self,
        parent_steps: Vec<ParentStep>,
        mut conditions: Vec<Condition>,
        id: String,
        _constants: HashMap<String, Value>,
        _documents: Documents,
    ) -> Result<LlmStep, LlmBuildingError> {
        for step in parent_steps {
            conditions.extend(step.conditions);
        }

        let text_message = LlmStep::TextMessage(TextMessage::new(id, self.message, self.hold, conditions));
        Ok(text_message)
    }
}
