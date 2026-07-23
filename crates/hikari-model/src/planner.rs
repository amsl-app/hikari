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

    /// Computes the date clients should treat this entry as due: `date`, unless the entry
    /// is unchecked and `date` is in the past, in which case it's `today`.
    #[must_use]
    pub fn with_effective_date(mut self, today: NaiveDate) -> Self {
        self.effective_date = if !self.completed && self.date < today {
            today
        } else {
            self.date
        };
        self
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

/// A milestone with its planner entries embedded, requested via the `deep` query param.
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
}

impl PlannerMilestone {
    #[must_use]
    pub fn as_milestone_full(&self, deep: bool, entries: Vec<PlannerEntry>) -> PlannerMilestoneFull {
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
            entries: if deep { entries } else { Vec::new() },
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn make_entry(date: NaiveDate, completed: bool) -> PlannerEntry {
        PlannerEntry {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            date,
            effective_date: date,
            title: "test".to_string(),
            completed,
            priority: 0,
            milestone_id: None,
            created_at: NaiveDateTime::default(),
            updated_at: NaiveDateTime::default(),
        }
    }

    #[test]
    fn test_effective_date_unchecked_past_becomes_today() {
        let today = NaiveDate::from_ymd_opt(2026, 7, 23).unwrap();
        let past = today - Duration::days(3);
        let entry = make_entry(past, false).with_effective_date(today);
        assert_eq!(entry.effective_date, today);
    }

    #[test]
    fn test_effective_date_checked_past_stays_date() {
        let today = NaiveDate::from_ymd_opt(2026, 7, 23).unwrap();
        let past = today - Duration::days(3);
        let entry = make_entry(past, true).with_effective_date(today);
        assert_eq!(entry.effective_date, past);
    }

    #[test]
    fn test_effective_date_future_stays_date() {
        let today = NaiveDate::from_ymd_opt(2026, 7, 23).unwrap();
        let future = today + Duration::days(3);
        let entry = make_entry(future, false).with_effective_date(today);
        assert_eq!(entry.effective_date, future);
    }

    #[test]
    fn test_effective_date_today_stays_date() {
        let today = NaiveDate::from_ymd_opt(2026, 7, 23).unwrap();
        let entry = make_entry(today, false).with_effective_date(today);
        assert_eq!(entry.effective_date, today);
    }
}
