use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum::EnumString;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, Serialize, ToSchema, Deserialize, EnumString)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case", ascii_case_insensitive)]
#[serde(deny_unknown_fields)]
pub enum ModuleInstanceStatus {
    #[default]
    NotStarted,
    Started,
    Finished,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, ToSchema, Deserialize)]
pub struct ModuleInstance {
    pub user_id: Uuid,
    pub module: String,
    pub status: ModuleInstanceStatus,
    pub completion: Option<DateTime<Utc>>,
}
