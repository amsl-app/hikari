use super::llm::PromptType;
use super::{LlmModel, Memory};
use crate::builder::error::LlmBuildingError;
use crate::builder::slot::SlotValuePair;
use crate::builder::slot::paths::SlotPath;
use crate::builder::steps::{
    Condition, Documents, Flow, InjectionTrait, IntoLlmStep, ParentStep, SlotsTrait, Template, load_prompt_and_temp,
};
use crate::builder::tools::Tool;
use crate::builder::{build_memory_filter, step_id_from_flow};
use crate::execution::core::LlmCore;
use crate::execution::steps::LlmStep;
use crate::execution::steps::conversation_validator::ConversationValidator;
use schemars::JsonSchema;
use serde::Deserialize;
use std::collections::HashMap;
use yaml_serde::Value;

const PROMPT_KEY: &str = "VALIDATOR_PREFIX";
const TEMPERATURE_KEY: &str = "VALIDATOR_TEMPERATURE";

#[derive(Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ConversationGoal {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: String,
    pub goal: Template,
    #[serde(default)]
    pub examples: Vec<Template>,
}

impl SlotsTrait for ConversationGoal {
    fn injection_slots(&self) -> Vec<SlotPath> {
        let mut slots = self.goal.injection_slots();
        slots.extend(self.examples.iter().flat_map(SlotsTrait::injection_slots));
        slots
    }
}

impl InjectionTrait for ConversationGoal {
    fn inject(&self, values: &[SlotValuePair]) -> Self {
        ConversationGoal {
            name: self.name.clone(),
            goal: self.goal.inject(values),
            examples: self.examples.iter().map(|e| e.inject(values)).collect(),
        }
    }
}

#[derive(Deserialize, Debug, Clone, Default, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum ValidationType {
    #[default]
    All,
    Any,
}

#[derive(Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ValidatorBuilder {
    pub goals: Vec<ConversationGoal>,
    #[serde(default)]
    pub prompts: Vec<PromptType>,
    #[serde(flatten, default)]
    pub memory: Memory,
    pub success: Flow,
    pub fail: Flow,
    #[serde(flatten)]
    pub model: LlmModel,
    #[serde(default)]
    pub skip_prefix: bool,
    #[serde(default, rename = "type")]
    /// # Whether all or any goals need to be fulfilled for a success
    pub validation_type: ValidationType,
}

impl SlotsTrait for ValidatorBuilder {
    fn injection_slots(&self) -> Vec<SlotPath> {
        let mut slots = self
            .goals
            .iter()
            .flat_map(SlotsTrait::injection_slots)
            .collect::<Vec<_>>();
        slots.extend(self.prompts.iter().flat_map(SlotsTrait::injection_slots));
        slots
    }
}

impl IntoLlmStep for ValidatorBuilder {
    fn into_llm_step(
        mut self,
        parent_steps: Vec<ParentStep>,
        mut conditions: Vec<Condition>,
        id: String,
        constants: HashMap<String, Value>,
        _documents: Documents,
    ) -> Result<LlmStep, LlmBuildingError> {
        self.prompts.iter_mut().for_each(|p| {
            p.insert_constant(&constants);
        });

        // insert_constants must be called before we extract the slots

        let slots: Vec<SlotPath> = self.injection_slots();

        let ValidatorBuilder {
            goals,
            mut prompts,
            memory: Memory {
                memory_limit,
                memory: memory_selection,
            },
            success,
            fail,
            model,
            skip_prefix,
            validation_type,
        } = self;

        let (prefix, temperature) = load_prompt_and_temp(&constants, PROMPT_KEY, TEMPERATURE_KEY)?;

        let goto_on_success = step_id_from_flow(success, &parent_steps);
        let goto_on_fail = step_id_from_flow(fail, &parent_steps);

        for step in parent_steps {
            conditions.extend(step.conditions);
        }

        if !skip_prefix {
            prompts.insert(0, PromptType::System(prefix.into()));
        }

        let memory_filter = build_memory_filter(&memory_selection, &id);
        let core = LlmCore::new(
            prompts,
            model.with_default_temperature(temperature),
            slots,
            memory_filter,
            memory_limit,
            Some(Tool::ValidationTool(goals)),
        );
        tracing::trace!(?goto_on_success, ?goto_on_fail, "Goto ");
        let conversation_validator = LlmStep::ConversationValidator(ConversationValidator::new(
            id,
            core,
            goto_on_success,
            goto_on_fail,
            conditions,
            validation_type,
        ));
        Ok(conversation_validator)
    }
}
