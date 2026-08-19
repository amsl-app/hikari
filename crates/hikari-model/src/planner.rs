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
    pub effective_date: NaiveDate,
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
    pub effective_date: NaiveDate,
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
            effective_date: self.effective_date,
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
pub struct Goal {
    pub id: Uuid,
    #[serde(skip_serializing)]
    pub user_id: Uuid,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub fulfilled: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NewGoal {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}

/// A goal with its milestones embedded, requested via the `deep` query param on list endpoints (always embedded for a single goal).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GoalFull {
    pub id: Uuid,
    #[serde(skip_serializing)]
    pub user_id: Uuid,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub fulfilled: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub milestones: Vec<PlannerMilestone>,
}

impl Goal {
    #[must_use]
    pub fn as_goal_full(&self, milestones: Vec<PlannerMilestone>) -> GoalFull {
        GoalFull {
            id: self.id,
            user_id: self.user_id,
            name: self.name.clone(),
            description: self.description.clone(),
            fulfilled: self.fulfilled,
            created_at: self.created_at,
            updated_at: self.updated_at,
            milestones,
        }
    }
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

/// A milestone with its planner entries and goals embedded, requested via the `deep` query param.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PlannerMilestoneFull {
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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub entries: Vec<PlannerEntry>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub goals: Vec<Goal>,
}

impl PlannerMilestone {
    #[must_use]
    pub fn as_milestone_full(&self, entries: Vec<PlannerEntry>, goals: Vec<Goal>) -> PlannerMilestoneFull {
        PlannerMilestoneFull {
            id: self.id,
            user_id: self.user_id,
            title: self.title.clone(),
            date: self.date,
            description: self.description.clone(),
            module_id: self.module_id.clone(),
            origin_id: self.origin_id.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
            entries,
            goals,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NewPlannerMilestone {
    pub title: String,
    pub date: NaiveDate,
    #[serde(default)]
    pub description: Option<String>,
    pub goals: Vec<Uuid>,
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
