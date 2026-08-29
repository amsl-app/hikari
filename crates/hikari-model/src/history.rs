use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, ToSchema)]
pub struct HistoryEntry {
    pub completed: DateTime<Utc>,
    #[serde(flatten)]
    pub value: HistoryEntryType,
}

#[derive(Serialize, ToSchema)]
#[serde(tag = "type")]
pub enum HistoryEntryType {
    Assessment(HistoryAssessment),
    Module(HistoryModule),
    Session(HistorySession),
}

#[derive(Serialize, ToSchema)]
pub struct HistoryAssessment {
    #[deprecated(note = "This field is not used and will be removed in a future version")]
    pub assessment_type: String,
    pub session_id: Uuid,
}

#[derive(Serialize, ToSchema)]
pub struct HistoryModule {
    pub module: String,
}

#[derive(Serialize, ToSchema)]
pub struct HistorySession {
    pub module: String,
    pub session: String,
}
