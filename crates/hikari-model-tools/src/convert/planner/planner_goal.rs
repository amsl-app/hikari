use crate::convert::FromDbModel;
use hikari_entity::planner::planner_goal::Model as GoalModel;
use hikari_model::planner::Goal;

impl FromDbModel<GoalModel> for Goal {
    fn from_db_model(model: GoalModel) -> Self {
        Self {
            id: model.id,
            user_id: model.user_id,
            name: model.name,
            description: model.description,
            fulfilled: model.fulfilled,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}
