use schemars::JsonSchema;
use serde::Deserialize;
use serde_yml::Value;
use std::collections::HashMap;

use crate::builder::build_memory_filter;
use crate::builder::error::LlmBuildingError;
use crate::builder::slot::paths::SlotPath;
use crate::builder::steps::llm::PromptType;
use crate::builder::steps::{Condition, Documents, IntoLlmStep, ParentStep, SlotsTrait, load_prompt_and_temp};
use crate::builder::tools::Tool;
use crate::execution::core::LlmCore;
use crate::execution::steps::LlmStep;
use crate::execution::steps::conversation_summarizer::ConversationSummarizer;

use super::{LlmModel, Memory};

const PROMPT_KEY: &str = "SUMMARIZER_PREFIX";
const TEMPERATURE_KEY: &str = "SUMMARIZER_TEMPERATURE";

#[derive(Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SummarizerBuilder {
    #[serde(default)]
    pub prompts: Vec<PromptType>,
    #[serde(default, flatten)]
    pub memory: Memory,
    #[serde(default, rename = "type")]
    pub update_type: UpdateType,
    #[serde(flatten)]
    pub model: LlmModel,
    #[serde(default)]
    pub skip_prefix: bool,
}

impl SlotsTrait for SummarizerBuilder {
    fn injection_slots(&self) -> Vec<SlotPath> {
        self.prompts
            .iter()
            .flat_map(super::SlotsTrait::injection_slots)
            .collect::<Vec<_>>()
    }
}

impl IntoLlmStep for SummarizerBuilder {
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

        let SummarizerBuilder {
            mut prompts,
            memory: Memory {
                memory_limit,
                memory: memory_selection,
            },
            update_type,
            model,
            skip_prefix,
        } = self;

        let (prefix, temperature) = load_prompt_and_temp(&constants, PROMPT_KEY, TEMPERATURE_KEY)?;

        if !skip_prefix {
            prompts.insert(0, PromptType::System(prefix.into()));
        }

        for step in parent_steps {
            conditions.extend(step.conditions);
        }

        let memory_filter = build_memory_filter(&memory_selection, &id);
        let temperature = model.temperature.unwrap_or(temperature);

        let core = LlmCore::new(
            prompts,
            model.with_default_temperature(temperature),
            slots,
            memory_filter,
            memory_limit,
            Some(Tool::Summarizer),
        );
        let conversation_summarizer =
            LlmStep::ConversationSummarizer(ConversationSummarizer::new(id, core, update_type, conditions));
        Ok(conversation_summarizer)
    }
}

#[derive(Deserialize, Debug, Clone, Default, Copy, Eq, PartialEq, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum UpdateType {
    #[serde(rename = "append")]
    #[default]
    Append,
    #[serde(rename = "replace")]
    Replace,
}
