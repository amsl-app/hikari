use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PlannerAssistantModule {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PlannerAssistantSession {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PlannerAssistantExistingEntry {
    pub date: NaiveDate,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PlannerAssistantRequest {
    pub text: String,
    /// Client's local date for resolving relative expressions like "tomorrow". Falls back to UTC if absent.
    #[serde(default)]
    pub today: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PlannerEntry {
    pub id: Uuid,
    #[serde(skip_serializing)]
    pub user_id: Uuid,
    pub date: NaiveDate,
    pub title: String,
    pub completed: bool,
    pub priority: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PlannerIcalToken {
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NewPlannerEntry {
    pub date: NaiveDate,
    pub title: String,
    pub priority: i32,
    #[serde(default)]
    pub module_id: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
}
