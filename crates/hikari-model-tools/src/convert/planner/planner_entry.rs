use crate::convert::FromDbModel;
use hikari_entity::planner::planner_entry::PlannerEntryWithEffectiveDate;
use hikari_model::planner::PlannerEntry;

impl FromDbModel<PlannerEntryWithEffectiveDate> for PlannerEntry {
    fn from_db_model(model: PlannerEntryWithEffectiveDate) -> Self {
        Self {
            id: model.id,
            user_id: model.user_id,
            date: model.date,
            effective_date: model.effective_date,
            title: model.title,
            completed: model.completed,
            priority: model.priority,
            milestone_id: model.milestone_id,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}
