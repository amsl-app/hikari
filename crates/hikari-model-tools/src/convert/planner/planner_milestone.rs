use crate::convert::FromDbModel;
use hikari_entity::planner::planner_milestone::Model as PlannerMilestoneModel;
use hikari_model::planner::PlannerMilestone;

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
