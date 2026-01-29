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

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum HistoryAssessmentType {
    Pre,
    Post,
}

#[derive(Serialize, ToSchema)]
pub struct HistoryAssessment {
    pub assessment_type: HistoryAssessmentType,
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
