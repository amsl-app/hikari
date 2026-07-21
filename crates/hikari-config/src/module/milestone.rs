use chrono::NaiveDate;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ModuleMilestone {
    /// # Stable identifier of the milestone within the module
    pub id: String,
    /// # Title of the milestone
    pub title: String,
    /// # Absolute target date of the milestone
    pub date: NaiveDate,
    /// # Optional description
    pub description: Option<String>,
}
