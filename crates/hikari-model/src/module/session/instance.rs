use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum::EnumString;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, Serialize, ToSchema, Deserialize, EnumString)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case", ascii_case_insensitive)]
#[serde(deny_unknown_fields)]
pub enum SessionInstanceStatus {
    #[default]
    NotStarted,
    Started,
    Finished,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, ToSchema, Deserialize)]
pub struct SessionInstance {
    pub user_id: Uuid,
    pub module: String,
    pub session: String,
    pub status: SessionInstanceStatus,
    pub bot_id: Option<String>,
    pub last_conv_id: Option<Uuid>,
    pub completion: Option<DateTime<Utc>>,
}
