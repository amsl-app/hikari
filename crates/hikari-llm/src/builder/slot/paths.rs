use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{builder::Selection, execution::error::LlmExecutionError};

#[derive(Deserialize, Debug, Clone, Serialize, Hash, Eq, PartialEq, Default, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum Destination {
    /// # Across all conversations and sessions
    Global,
    #[default]
    /// # Within the current
    Conversation,
    /// # Within the current session
    Session,
    /// # Within the current module
    Module,
}
#[derive(Deserialize, Debug, Clone, Serialize, Hash, Eq, PartialEq, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SlotPath {
    /// # Name of the slot
    pub name: String,
    #[serde(default)]
    /// # Depricated: Use destination instead
    pub global: Option<bool>,
    #[serde(default)]
    /// # Destination of the slot
    pub destination: Destination,
}

impl std::fmt::Display for SlotPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.destination {
            Destination::Conversation => write!(f, "conversation.{}", self.name),
            Destination::Session => write!(f, "session.{}", self.name),
            Destination::Module => write!(f, "module.{}", self.name),
            Destination::Global => write!(f, "global.{}", self.name),
        }
    }
}

impl SlotPath {
    #[must_use]
    pub fn new(name: String, destination: Destination) -> Self {
        Self {
            name,
            global: None,
            destination,
        }
    }

    #[must_use]
    pub fn destination(&self) -> &Destination {
        if self.global.unwrap_or(false) {
            &Destination::Global
        } else {
            &self.destination
        }
    }
}
#[derive(Deserialize, Debug, Clone, Default, Hash, Eq, PartialEq, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct UserPath {
    /// # `JsonPath` to values of the user profile
    pub path: String,
}

#[derive(Deserialize, Debug, Clone, Default, Hash, Eq, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ModulePath {
    /// # `JsonPath` to values of the module config
    pub path: String,
    #[serde(default)]
    /// # Selection of the module to load from
    /// Mostly current module and can be left
    pub module: Selection<ModuleSessionSelector>,
}

#[derive(Deserialize, Debug, Clone, Default, Hash, Eq, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SessionPath {
    /// # `JsonPath` to values of the session config
    pub path: String,
    /// # Selection of the module to load from
    /// Mostly current module and can be left
    #[serde(default)]
    pub module: Selection<ModuleSessionSelector>,
    /// # Selection of the session of the module to load from
    /// Mostly current session and can be left
    #[serde(default)]
    pub session: Selection<ModuleSessionSelector>,
}

#[derive(Deserialize, Debug, Clone, Default, Hash, Eq, PartialEq, Serialize, JsonSchema)]
pub enum ModuleSessionSelector {
    #[default]
    Current,
}

impl Selection<ModuleSessionSelector> {
    pub fn get_id(&self, current_id: &str) -> Result<String, LlmExecutionError> {
        match &self {
            Selection::Selector(selector_type) => match selector_type {
                ModuleSessionSelector::Current => Ok(current_id.to_owned()),
            },
            Selection::From(id) => Ok(id.clone()),
        }
    }
}
