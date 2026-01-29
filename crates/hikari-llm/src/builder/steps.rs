use super::Selection;
use crate::builder::error::LlmBuildingError;
use crate::builder::slot::SlotValuePair;
use crate::builder::slot::paths::{Destination, SlotPath};
use crate::builder::steps::api::ApiBuilder;
use crate::builder::steps::counter::CounterBuilder;
use crate::builder::steps::extractor::ExtractorBuilder;
use crate::builder::steps::llm::LlmBuilder;
use crate::builder::steps::retriever::RetrieverBuilder;
use crate::builder::steps::sse::SseBuilder;
use crate::builder::steps::summarizer::SummarizerBuilder;
use crate::builder::steps::validator::ValidatorBuilder;
use crate::execution::steps::LlmStep;
use crate::execution::steps::combined_step::CombinedStep;
use hikari_utils::values::ValueDecoder;
use indexmap::{IndexMap, indexmap};
use llm::MemorySelector;
use message::MessageBuilder;
use nonempty::NonEmpty;
use num_traits::cast::ToPrimitive;
use regex::Regex;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_yml::Value;
use set_slot::SetSlotBuilder;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, LazyLock};
use tokio::sync::Mutex;

pub mod api;
pub mod counter;
pub mod extractor;
pub mod flow;
pub mod llm;
pub mod message;
pub mod retriever;
pub mod set_slot;
pub mod sse;
pub mod summarizer;
pub mod validator;

static TEMPLATE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{\{(\w+)}}").expect("template regex is invalid"));

#[derive(Debug, Clone, Default)]
pub struct Documents {
    pub primary: Vec<String>,
    pub secondary: Vec<String>,
}

impl Documents {
    #[must_use]
    pub fn new(primary: Vec<String>, secondary: Vec<String>) -> Self {
        Self { primary, secondary }
    }

    pub fn extend(&mut self, other: Documents) {
        self.primary.extend(other.primary);
        self.secondary.extend(other.secondary);
    }
}

pub trait IntoLlmStep {
    fn into_llm_step(
        self,
        parent_steps: Vec<ParentStep>,
        conditions: Vec<Condition>,
        id: String,
        constants: HashMap<String, Value>,
        documents: Documents,
    ) -> Result<LlmStep, LlmBuildingError>;
}

#[derive(Deserialize, Debug, Clone, Default, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct LlmModel {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

impl LlmModel {
    fn with_default_temperature(self, temp: f32) -> Self {
        Self {
            temperature: self.temperature.or(Some(temp)),
            model: self.model,
        }
    }
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct StepBuilder {
    /// # Unique identifier for the step
    /// Used to trace execution and reference steps
    pub id: String,
    #[serde(flatten)]
    /// # The type of step to executes
    pub step: StepType,
    #[serde(default)]
    /// # Conditions to evaluate before executing the step
    /// If any condition fails, the step is skipped
    pub conditions: Vec<Condition>,
}

impl StepBuilder {
    fn as_parent(&self) -> ParentStep {
        let steps = match &self.step {
            StepType::Chain(steps) | StepType::Combined(steps) => {
                steps.into_iter().map(|step| step.id.clone()).collect()
            }
            _ => vec![],
        };
        ParentStep {
            id: self.id.clone(),
            steps,
            conditions: self.conditions.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParentStep {
    pub id: String,
    pub steps: Vec<String>,
    pub conditions: Vec<Condition>,
}

impl StepBuilder {
    pub(crate) fn into_llm_step(
        self,
        parent_steps: Vec<ParentStep>,
        constants: HashMap<String, Value>,
        documents: Documents,
    ) -> Result<IndexMap<String, Arc<Mutex<LlmStep>>>, LlmBuildingError> {
        let step = self.into_raw_llm_step(parent_steps, constants, documents)?;
        step.into_iter()
            .map(|(k, v)| {
                let v = Arc::new(Mutex::new(v));
                Ok((k, v))
            })
            .collect()
    }

    fn create_chain(
        parent_step: ParentStep,
        mut parent_steps: Vec<ParentStep>,
        constants: HashMap<String, Value>,
        documents: Documents,
        mut chain: NonEmpty<Box<StepBuilder>>,
    ) -> Result<IndexMap<String, LlmStep>, LlmBuildingError> {
        parent_steps.push(parent_step);
        let map = if let Some(last) = chain.pop() {
            let mut map = IndexMap::new();
            for step in chain {
                map.extend(step.into_raw_llm_step(parent_steps.clone(), constants.clone(), documents.clone())?);
            }
            map.extend(last.into_raw_llm_step(parent_steps, constants, documents)?);
            map
        } else {
            // We did not pop a chain element => the chain is only one step => no need to iterate
            chain.head.into_raw_llm_step(parent_steps, constants, documents)?
        };

        Ok(map)
    }

    pub fn into_raw_llm_step(
        self,
        parent_steps: Vec<ParentStep>,
        constants: HashMap<String, Value>,
        documents: Documents,
    ) -> Result<IndexMap<String, LlmStep>, LlmBuildingError> {
        let parent_step = self.as_parent();
        match self.step {
            StepType::Llm(llm) => create_step(llm, parent_steps, self.conditions, self.id, constants, documents),
            StepType::Summarizer(summarizer) => {
                create_step(summarizer, parent_steps, self.conditions, self.id, constants, documents)
            }
            StepType::Validator(validator) => {
                create_step(validator, parent_steps, self.conditions, self.id, constants, documents)
            }
            StepType::Extractor(extractor) => {
                create_step(extractor, parent_steps, self.conditions, self.id, constants, documents)
            }
            StepType::Retriever(retriever) => {
                create_step(retriever, parent_steps, self.conditions, self.id, constants, documents)
            }
            StepType::Message(message) => {
                create_step(message, parent_steps, self.conditions, self.id, constants, documents)
            }
            StepType::SetSlot(set_slot) => {
                create_step(set_slot, parent_steps, self.conditions, self.id, constants, documents)
            }
            StepType::ApiCall(api) => create_step(api, parent_steps, self.conditions, self.id, constants, documents),
            StepType::SseCall(sse) => create_step(sse, parent_steps, self.conditions, self.id, constants, documents),
            StepType::Counter(counter) => {
                create_step(counter, parent_steps, self.conditions, self.id, constants, documents)
            }
            StepType::Flow(flow) => create_step(flow, parent_steps, self.conditions, self.id, constants, documents),
            StepType::Chain(chain) => {
                let map = Self::create_chain(parent_step, parent_steps, constants, documents, chain)?;
                Ok(map)
            }
            StepType::Combined(combined) => {
                let map = Self::create_chain(parent_step, parent_steps, constants, documents, combined)?;
                let vec = map.into_values().collect();
                let combined = CombinedStep::new(self.id.clone(), vec, self.conditions);
                Ok(indexmap! {self.id => LlmStep::CombinedStep(combined)})
            }
        }
    }
}

fn create_step<T: IntoLlmStep>(
    step: T,
    parent_steps: Vec<ParentStep>,
    conditions: Vec<Condition>,
    id: String,
    constants: HashMap<String, Value>,
    documents: Documents,
) -> Result<IndexMap<String, LlmStep>, LlmBuildingError> {
    let step: LlmStep = step.into_llm_step(parent_steps, conditions, id.clone(), constants, documents)?;

    Ok(indexmap! { id => step })
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum StepType {
    /// # Step that sends a static message
    Message(MessageBuilder),
    /// # Step that sends a llm message to the user
    /// NOTE: Messages are directly sent to the user and can also be stored optionally into slots
    Llm(LlmBuilder),
    /// # Step that chains multiple steps sequentially
    Chain(#[schemars(with = "Vec<StepBuilder>")] NonEmpty<Box<StepBuilder>>),
    /// # Step that combines multiple steps and executes them in parallel
    /// Maybe have intereference and should be used with care
    Combined(#[schemars(with = "Vec<StepBuilder>")] NonEmpty<Box<StepBuilder>>),
    /// # Step that summarizes the current conversation and store it into the slot 'summary'
    Summarizer(SummarizerBuilder),
    /// # Step that validate the current conversation against a described goal
    Validator(ValidatorBuilder),
    /// # Step that extracts information from the conversation and store it into slots
    Extractor(ExtractorBuilder),
    /// # Step that retrieves documents from a vector store and store them into slots
    Retriever(RetrieverBuilder),
    /// # Step that makes an API call
    ApiCall(ApiBuilder),
    /// # Step that makes a Server-Sent Events (SSE) call
    SseCall(SseBuilder),
    /// # Step that sets a slot value manually
    SetSlot(SetSlotBuilder),
    /// # Step that increments or decrements a counter
    Counter(CounterBuilder),
    /// # Step that controls the flow of execution
    /// Here we can continue, repeat or goto other steps
    Flow(flow::FlowBuilder),
}

// Only used for take() in StepBuilder::into_llm_step
impl Default for StepType {
    fn default() -> Self {
        StepType::Llm(LlmBuilder::default())
    }
}

#[derive(Debug, Clone, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Memory {
    #[serde(default)]
    /// # Memory selection for this step
    /// Include messages for the memory only from the current step, specific steps, or all steps
    pub memory: Vec<Selection<MemorySelector>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// # Limit of memory entries to consider
    pub memory_limit: Option<usize>,
}

#[derive(Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Condition {
    #[serde(flatten)]
    /// # Slot to evaluate the condition against
    pub slot: SlotPath,
    /// # Condition operation to perform
    pub condition: ConditionOperation,
}

#[derive(Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum ConditionOperation {
    Equals(#[schemars(with = "serde_json::Value")] Value),
    NotEquals(#[schemars(with = "serde_json::Value")] Value),
    Exists(bool),
    GreaterThan(f64),
    LessThan(f64),
    GreaterThanOrEqual(f64),
    LessThanOrEqual(f64),
}

#[derive(Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum Flow {
    Action(Next),
    /// # Goto another step by its ID
    /// NOTE: IDs of Chains steps are not allowed here. Insted use the ID of the first step in the chain
    Goto(String),
}

#[derive(Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum Next {
    /// # Continues to the next step in the flow
    Continue,
    /// # Repeats the current chain
    /// That means if the current step is part of a chain, the whole chain is repeated
    Repeat,
}

pub(crate) fn load_prompt_and_temp<'a>(
    constants: &'a HashMap<String, Value>,
    prompt_key: &'a str,
    temp_key: &'a str,
) -> Result<(&'a str, f32), LlmBuildingError> {
    let prefix = load_prompt(constants, prompt_key)?;
    let temperature = load_temp(constants, temp_key)?;
    Ok((prefix, temperature))
}

pub(crate) fn load_prompt<'a>(
    constants: &'a HashMap<String, Value>,
    prompt_key: &'a str,
) -> Result<&'a str, LlmBuildingError> {
    let prefix = constants
        .get(prompt_key)
        .ok_or(LlmBuildingError::MissingPrefix(prompt_key.to_string()))?;
    let prefix = prefix
        .as_str()
        .ok_or(LlmBuildingError::ExpectedString(prompt_key.to_string()))?;
    Ok(prefix)
}

pub(crate) fn load_temp<'a>(constants: &'a HashMap<String, Value>, temp_key: &'a str) -> Result<f32, LlmBuildingError> {
    let temperature = constants
        .get(temp_key)
        .ok_or(LlmBuildingError::MissingPrefix(temp_key.to_string()))?
        .as_f64()
        .and_then(|v| v.to_f32())
        .ok_or(LlmBuildingError::ExpectedFloat(temp_key.to_string()))?;
    Ok(temperature)
}

#[derive(Debug, Clone, Deserialize, Hash, Eq, PartialEq)]
pub enum Placeholder {
    Global(String),
    Module(String),
    Session(String),
    Conversation(String),
    Default(String),
}

impl Placeholder {
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        let parts: Vec<&str> = raw.split('.').collect();
        tracing::trace!(?parts, "Parsing placeholder");
        let (key, value) = match parts.as_slice() {
            [default] => return Some(Placeholder::Default(default.to_string())),
            [key, value] => (*key, value.to_string()),
            _ => return None,
        };

        match key {
            "global" => Some(Placeholder::Global(value)),
            "module" => Some(Placeholder::Module(value)),
            "session" => Some(Placeholder::Session(value)),
            "conversation" => Some(Placeholder::Conversation(value)),
            _ => None,
        }
    }
}

impl From<Placeholder> for SlotPath {
    fn from(placeholder: Placeholder) -> Self {
        match placeholder {
            Placeholder::Global(key) => SlotPath::new(key, Destination::Global),
            Placeholder::Module(key) => SlotPath::new(key, Destination::Module),
            Placeholder::Session(key) => SlotPath::new(key, Destination::Session),
            Placeholder::Conversation(key) | Placeholder::Default(key) => SlotPath::new(key, Destination::Conversation),
        }
    }
}

pub trait SlotsTrait {
    fn injection_slots(&self) -> Vec<SlotPath>;
}

pub trait InjectionTrait: SlotsTrait {
    #[must_use]
    fn inject(&self, values: &[SlotValuePair]) -> Self;
}

#[derive(Deserialize, Debug, Clone, JsonSchema)]
/// # A template that can contain placeholders for slot injection
/// Placeholders are defined using the syntax `{{destination.slot_name}}`
/// Destinations can be `global`, `module`, `session`, `conversation`
/// If no destination is provided, `conversation` is used as default
pub struct Template(#[schemars(with = "serde_json::Value")] pub Value);

impl AsRef<Value> for Template {
    fn as_ref(&self) -> &Value {
        &self.0
    }
}

impl std::fmt::Display for Template {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.encode())
    }
}

impl From<&str> for Template {
    fn from(value: &str) -> Self {
        Template(Value::decode(value))
    }
}

impl From<String> for Template {
    fn from(value: String) -> Self {
        Template(Value::decode(&value))
    }
}

impl From<Value> for Template {
    fn from(value: Value) -> Self {
        Template(value)
    }
}

impl Template {
    fn placeholders(&self) -> Vec<Placeholder> {
        let string = self.0.encode();
        // Get all matches
        TEMPLATE
            .captures_iter(&string)
            .filter_map(|cap| cap.get(1).map(|m| Placeholder::parse(m.as_str())))
            .flatten()
            .collect()
    }
}
impl SlotsTrait for Template {
    fn injection_slots(&self) -> Vec<SlotPath> {
        self.placeholders().into_iter().map(SlotPath::from).collect()
    }
}

impl InjectionTrait for Template {
    fn inject(&self, values: &[SlotValuePair]) -> Template {
        let mut content = self.0.encode();
        tracing::trace!(?content, "Injecting values into template");

        for placeholder in self.placeholders() {
            tracing::trace!(?placeholder, "Found placeholder");
            let key = match &placeholder {
                Placeholder::Global(k) => format!("global.{k}"),
                Placeholder::Module(k) => format!("module.{k}"),
                Placeholder::Session(k) => format!("session.{k}"),
                Placeholder::Conversation(k) => format!("conversation.{k}"),
                Placeholder::Default(k) => k.clone(),
            };
            let key = format!("{{{{{key}}}}}"); // We need twice as much curly braces to escape them in the string

            let slot_path = SlotPath::from(placeholder.clone());

            let value = values
                .iter()
                .find(|v| v.path == slot_path)
                .map_or("**N/A**".to_string(), |v| v.value.0.encode());

            content = content.replace(&key, &value);
            tracing::trace!(%value, %key, "Replaced placeholder with value");
        }

        Template(Value::decode(&content))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_deserialization() {
        let StepBuilder { id, step, .. } = serde_json::from_str(
            r#"{
                    "id": "test",
                    "llm": {
                        "prompts": [
                            { "system": { "message": "..." , "inputs": []}},
                            { "system": { "message": "..." , "inputs": []}}
                        ]
                    }
                }"#,
        )
        .unwrap();
        assert_eq!(id, "test");
        let StepType::Llm(llm) = step else {
            panic!("Expected Conditions, got {step:?}")
        };
        let LlmBuilder { prompts, .. } = llm;
        assert_eq!(prompts.len(), 2);
    }
}
