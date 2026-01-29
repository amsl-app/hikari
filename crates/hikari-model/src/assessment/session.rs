use chrono::{DateTime, Utc};
use hikari_config::assessment::Assessment;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[repr(u16)]
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, Serialize, ToSchema, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    #[default]
    NotStarted = 1,
    Running = 2,
    Finished = 3,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AssessmentSession {
    pub session_id: Uuid,
    pub assessment: Assessment,
    pub status: Status,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed: Option<DateTime<Utc>>,
}
