use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Default, Serialize, Deserialize, Clone, ToSchema, Debug, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct JournalFocusEntry {
    /// # Name of the journal focus
    pub name: String,
    /// # Icon associated with the journal focus
    pub icon: String,
}

#[derive(Default, Serialize, Deserialize, Clone, ToSchema, Debug, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct JournalConfig {
    #[serde(default)]
    /// # List of focus which are available in the journal
    /// Focusses assign tags and icons to journal entries
    pub focus: Vec<JournalFocusEntry>,
}
