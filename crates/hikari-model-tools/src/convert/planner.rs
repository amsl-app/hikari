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
