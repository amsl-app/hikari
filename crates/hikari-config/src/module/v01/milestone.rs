use chrono::NaiveDate;
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ModuleMilestoneV01 {
    /// # Stable identifier of the milestone within the module
    pub(crate) id: String,
    /// # Title of the milestone
    pub(crate) title: String,
    /// # Absolute target date of the milestone
    pub(crate) date: NaiveDate,
    /// # Optional description
    pub(crate) description: Option<String>,
}
