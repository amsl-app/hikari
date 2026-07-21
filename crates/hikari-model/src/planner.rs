use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

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
    pub milestone_id: Option<Uuid>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

/// A planner entry with its milestone embedded (instead of just the milestone id) to save extra lookups.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PlannerEntryFull {
    pub id: Uuid,
    #[serde(skip_serializing)]
    pub user_id: Uuid,
    pub date: NaiveDate,
    pub title: String,
    pub completed: bool,
    pub priority: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub milestone: Option<PlannerMilestone>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl PlannerEntry {
    #[must_use]
    pub fn as_entry_full(&self, milestone: Option<PlannerMilestone>) -> PlannerEntryFull {
        PlannerEntryFull {
            id: self.id,
            user_id: self.user_id,
            date: self.date,
            title: self.title.clone(),
            completed: self.completed,
            priority: self.priority,
            milestone,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
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
    pub milestone_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PlannerMilestone {
    pub id: Uuid,
    #[serde(skip_serializing)]
    pub user_id: Uuid,
    pub title: String,
    pub date: NaiveDate,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin_id: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NewPlannerMilestone {
    pub title: String,
    pub date: NaiveDate,
    #[serde(default)]
    pub description: Option<String>,
}

/// A module-defined milestone the user may import, annotated with whether it is already present.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ImportableMilestone {
    pub id: String,
    pub title: String,
    pub date: NaiveDate,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub already_imported: bool,
}
