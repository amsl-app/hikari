use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use utoipa::ToSchema;

#[derive(Deserialize, Clone, Debug, Default, Serialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ModuleConfig {
    pub groups: HashSet<ModuleGroup>,
}

impl ModuleConfig {
    #[must_use]
    pub fn groups(&self) -> &HashSet<ModuleGroup> {
        &self.groups
    }

    #[must_use]
    pub fn ids(&self) -> HashSet<&String> {
        self.groups.iter().map(|group| &group.key).collect()
    }
}

#[derive(Deserialize, Clone, Debug, Serialize, ToSchema, JsonSchema, PartialEq, Eq, Hash)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ModuleGroup {
    /// # Key of the module group
    /// The key is referenced by modules to assign them to a group
    pub key: String,
    /// # Label of the module group
    /// Label is used for display purposes in the frontend
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// # Weight of the module group
    /// Used to order module groups in the frontend. Higher weight means higher up in the list
    pub weight: Option<usize>,
}
