use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};
use strum::{AsRefStr, Display};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, Display, AsRefStr)]
pub enum Priority {
    #[serde(rename = "LOW", alias = "low", alias = "Low")]
    Low,
    #[serde(rename = "MEDIUM", alias = "medium", alias = "Medium")]
    Medium,
    #[serde(rename = "HIGH", alias = "high", alias = "High")]
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PlannerEntry {
    pub id: Uuid,
    #[serde(skip_serializing)]
    pub user_id: Uuid,
    pub date: NaiveDate,
    pub title: String,
    pub completed: bool,
    pub priority: Priority,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NewPlannerEntry {
    pub date: NaiveDate,
    pub title: String,
    pub priority: Priority,
    #[serde(default)]
    pub module_id: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
}
