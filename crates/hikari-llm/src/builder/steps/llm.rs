use crate::builder::build_memory_filter;
use crate::builder::error::LlmBuildingError;
use crate::builder::slot::paths::SlotPath;
use crate::builder::slot::{SaveTarget, SlotValuePair};
use crate::builder::steps::{Condition, Documents, InjectionTrait, IntoLlmStep, ParentStep, SlotsTrait, Template};
use crate::execution::core::LlmCore;
use crate::execution::steps::LlmStep;
use crate::execution::steps::message_generator::MessageGenerator;
use async_openai::types::{
    ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestAssistantMessageContent,
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestSystemMessageContent,
    ChatCompletionRequestUserMessageArgs, ChatCompletionRequestUserMessageContent,
};
use hikari_model::llm::message::ConversationMessage;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_yml::Value;
use std::collections::HashMap;

use super::{LlmModel, Memory};

#[derive(Deserialize, Debug, Clone, Default, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct LlmBuilder {
    pub prompts: Vec<PromptType>,
    /// # Whether to hold the conversation after this message
    /// Hold means the user can send a response before continuing
    #[serde(default)]
    pub hold: bool,
    #[serde(default, flatten)]
    pub memory: Memory,
    #[serde(flatten)]
    pub model: LlmModel,
    #[serde(default)]
    #[deprecated(note = "LLM_PREFIX was already empty. Use constants and prompts instead.")]
    /// # Deprecated
    pub skip_prefix: bool,
    #[serde(default)]
    pub store: Option<SaveTarget>,
}

impl SlotsTrait for LlmBuilder {
    fn injection_slots(&self) -> Vec<SlotPath> {
        let mut slots = self
            .prompts
            .iter()
            .flat_map(super::SlotsTrait::injection_slots)
            .collect::<Vec<_>>();
        slots.extend(self.prompts.iter().flat_map(super::SlotsTrait::injection_slots));
        slots
    }
}

impl IntoLlmStep for LlmBuilder {
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

        let LlmBuilder {
            prompts,
            hold,
            memory: Memory {
                memory_limit,
                memory: memory_selector,
            },
            model,
            store,
            .. // skip_prefix is deprecated and not used
        } = self;

        for step in parent_steps {
            conditions.extend(step.conditions);
        }

        let memory_filter = build_memory_filter(&memory_selector, &id);

        let core = LlmCore::new(prompts, model, slots, memory_filter, memory_limit, None);
        let message_generator = LlmStep::MessageGenerator(MessageGenerator::new(id, core, hold, conditions, store));
        Ok(message_generator)
    }
}

#[derive(Deserialize, Debug, Clone, Default, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum MemorySelector {
    #[default]
    Current,
    All,
}

#[derive(Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum PromptType {
    System(Template),
    User(Template),
    AI(Template),
    Constant(String),
}

impl PromptType {
    pub fn insert_constant(&mut self, constants: &HashMap<String, Value>) {
        if let PromptType::Constant(path) = self {
            if let Some(value) = constants.get(path) {
                *self = PromptType::System(Template::from(value.clone()));
            } else {
                tracing::warn!("Constant '{}' not found in provided constants", path);
            }
        }
    }
}

impl From<ConversationMessage> for PromptType {
    fn from(message: ConversationMessage) -> Self {
        let ConversationMessage { message, direction, .. } = message;
        let content = message.message_string().unwrap_or_default();
        match direction {
            hikari_model::chat::Direction::Receive => PromptType::User(Template::from(content)),
            hikari_model::chat::Direction::Send => PromptType::AI(Template::from(content)),
        }
    }
}

impl TryFrom<PromptType> for ChatCompletionRequestMessage {
    type Error = LlmBuildingError;

    fn try_from(value: PromptType) -> Result<Self, Self::Error> {
        match value {
            PromptType::System(template) => {
                let system = ChatCompletionRequestSystemMessageArgs::default()
                    .content(ChatCompletionRequestSystemMessageContent::Text(template.to_string()))
                    .build()?;
                Ok(ChatCompletionRequestMessage::System(system))
            }
            PromptType::User(template) => {
                let user = ChatCompletionRequestUserMessageArgs::default()
                    .content(ChatCompletionRequestUserMessageContent::Text(template.to_string()))
                    .build()?;
                Ok(ChatCompletionRequestMessage::User(user))
            }
            PromptType::AI(template) => {
                let ai = ChatCompletionRequestAssistantMessageArgs::default()
                    .content(ChatCompletionRequestAssistantMessageContent::Text(template.to_string()))
                    .build()?;
                Ok(ChatCompletionRequestMessage::Assistant(ai))
            }
            PromptType::Constant(_) => Err(LlmBuildingError::MissedFormatation(
                "Cannot convert Constant to ChatCompletionRequestMessage".to_string(),
            )),
        }
    }
}

impl SlotsTrait for PromptType {
    fn injection_slots(&self) -> Vec<SlotPath> {
        match self {
            PromptType::System(template) | PromptType::User(template) | PromptType::AI(template) => {
                template.injection_slots()
            }
            PromptType::Constant(_) => {
                // Constants do not have slots, but we return an empty vector to satisfy the trait
                vec![]
            }
        }
    }
}

impl InjectionTrait for PromptType {
    fn inject(&self, values: &[SlotValuePair]) -> Self {
        match self {
            PromptType::System(template) => PromptType::System(template.inject(values)),
            PromptType::User(template) => PromptType::User(template.inject(values)),
            PromptType::AI(template) => PromptType::AI(template.inject(values)),
            PromptType::Constant(path) => PromptType::Constant(path.clone()),
        }
    }
}
