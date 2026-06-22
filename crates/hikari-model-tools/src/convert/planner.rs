use crate::convert::FromDbModel;
use hikari_entity::planner_entry::Model as PlannerEntryModel;
use hikari_model::planner::PlannerEntry;

impl FromDbModel<PlannerEntryModel> for PlannerEntry {
    fn from_db_model(model: PlannerEntryModel) -> Self {
        Self {
            id: model.id,
            user_id: model.user_id,
            date: model.date,
            title: model.title,
            completed: model.completed,
            priority: model.priority,
            module_id: model.module_id,
            session_id: model.session_id,
            created_at: model.created_at,
        }
    }
}
