use schemars::JsonSchema;
use serde::Deserialize;

use crate::builder::{
    slot::paths::{ModulePath, SessionPath, SlotPath, UserPath},
    steps::{InjectionTrait, SlotsTrait, Template},
};

pub mod paths;

#[derive(Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum ValueSource {
    /// # Load value from the session config, like the content
    /// NOTE: The whole contents object which are referred inside of sessions can be accessed, e.g. path: $.contents[*].title
    Session(SessionPath),
    /// # Load value from a module's config
    /// NOTE: Contents cannot be accessed from modules here, only from the sessions
    Module(ModulePath),
    /// # Load value from user profile
    User(UserPath),
    /// # Load value from the user config
    /// User configs are custom objects
    UserConfig(UserPath),
}
#[derive(Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct LoadToSlot {
    /// # Name of the slot to load the value into
    pub name: String,
    /// # Source of the value to load
    pub source: ValueSource,
}
#[derive(Deserialize, Debug, Clone, Hash, Eq, PartialEq, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum SaveTarget {
    Slot(SlotPath),
}

#[derive(Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SlotValuePair {
    pub path: SlotPath,
    pub value: Template,
}

impl SlotsTrait for SlotValuePair {
    fn injection_slots(&self) -> Vec<SlotPath> {
        self.value.injection_slots()
    }
}

impl InjectionTrait for SlotValuePair {
    fn inject(&self, values: &[SlotValuePair]) -> Self {
        let value = self.value.inject(values);
        Self {
            path: self.path.clone(),
            value,
        }
    }
}
