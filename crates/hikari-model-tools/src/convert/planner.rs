use crate::convert::{FromDbModel, IntoDbModel};
use hikari_entity::planner_entry::Model as PlannerEntryModel;
use hikari_entity::planner_entry::Priority as PriorityModel;
use hikari_model::planner::PlannerEntry;
use hikari_model::planner::Priority;

impl FromDbModel<PriorityModel> for Priority {
    fn from_db_model(model: PriorityModel) -> Self {
        match model {
            PriorityModel::Low => Self::Low,
            PriorityModel::Medium => Self::Medium,
            PriorityModel::High => Self::High,
        }
    }
}

impl IntoDbModel<PriorityModel> for Priority {
    fn into_db_model(self) -> PriorityModel {
        match self {
            Self::Low => PriorityModel::Low,
            Self::Medium => PriorityModel::Medium,
            Self::High => PriorityModel::High,
        }
    }
}

impl FromDbModel<PlannerEntryModel> for PlannerEntry {
    fn from_db_model(model: PlannerEntryModel) -> Self {
        Self {
            id: model.id,
            user_id: model.user_id,
            date: model.date,
            title: model.title,
            completed: model.completed,
            priority: FromDbModel::from_db_model(model.priority),
            module_id: model.module_id,
            session_id: model.session_id,
            created_at: model.created_at,
        }
    }
}
