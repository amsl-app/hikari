use crate::convert::FromDbModel;
use hikari_entity::planner_entry::Model as PlannerEntryModel;
use hikari_entity::planner_milestone::Model as PlannerMilestoneModel;
use hikari_model::planner::{PlannerEntry, PlannerMilestone};

impl FromDbModel<PlannerEntryModel> for PlannerEntry {
    fn from_db_model(model: PlannerEntryModel) -> Self {
        Self {
            id: model.id,
            user_id: model.user_id,
            date: model.date,
            title: model.title,
            completed: model.completed,
            priority: model.priority,
            milestone_id: model.milestone_id,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}

impl FromDbModel<PlannerMilestoneModel> for PlannerMilestone {
    fn from_db_model(model: PlannerMilestoneModel) -> Self {
        Self {
            id: model.id,
            user_id: model.user_id,
            title: model.title,
            date: model.date,
            description: model.description,
            module_id: model.module_id,
            origin_id: model.origin_id,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::convert::FromDbModel;
    use chrono::NaiveDate;
    use sea_orm::prelude::*;

    #[test]
    fn converts_milestone() {
        let model = hikari_entity::planner_milestone::Model {
            id: Uuid::nil(),
            user_id: Uuid::nil(),
            title: "Exam".to_owned(),
            date: NaiveDate::from_ymd_opt(2026, 8, 1).unwrap(),
            description: Some("d".to_owned()),
            module_id: Some("mod-a".to_owned()),
            origin_id: Some("exam-1".to_owned()),
            created_at: Default::default(),
            updated_at: Default::default(),
        };
        let dto = hikari_model::planner::PlannerMilestone::from_db_model(model);
        assert_eq!(dto.title, "Exam");
        assert_eq!(dto.origin_id.as_deref(), Some("exam-1"));
    }
}
