use std::collections::HashMap;

use schemars::JsonSchema;
use serde::Deserialize;
use serde_yml::Value;

use super::{Condition, IntoLlmStep, ParentStep};
use crate::{
    builder::{error::LlmBuildingError, slot::SaveTarget, steps::Documents},
    execution::steps::{LlmStep, counter::Counter},
};

#[derive(Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct CounterBuilder {
    #[serde(flatten)]
    pub slot: SaveTarget,
}

impl IntoLlmStep for CounterBuilder {
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

        Ok(LlmStep::Counter(Counter::new(id, self.slot, conditions)))
    }
}
