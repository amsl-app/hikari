use std::collections::HashMap;

use schemars::JsonSchema;
use serde::Deserialize;
use serde_yml::Value;

use super::{Condition, IntoLlmStep, ParentStep};
use crate::builder::step_id_from_flow;
use crate::builder::steps::{Documents, Flow};
use crate::{
    builder::error::LlmBuildingError,
    execution::steps::{LlmStep, go_to::GoTo},
};

#[derive(Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct FlowBuilder {
    #[serde(flatten)]
    pub flow: Flow,
}

impl IntoLlmStep for FlowBuilder {
    fn into_llm_step(
        self,
        parent_steps: Vec<ParentStep>,
        mut conditions: Vec<Condition>,
        id: String,
        _constants: HashMap<String, Value>,
        _documents: Documents,
    ) -> Result<LlmStep, LlmBuildingError> {
        let next_step = step_id_from_flow(self.flow.clone(), &parent_steps);

        for step in parent_steps {
            conditions.extend(step.conditions);
        }

        Ok(LlmStep::GoTo(GoTo::new(id, next_step, conditions)))
    }
}
