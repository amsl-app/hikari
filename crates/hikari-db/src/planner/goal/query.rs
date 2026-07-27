use hikari_entity::planner::planner_goal::{Entity as Goal, Model as GoalModel};
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QueryOrder};
use uuid::Uuid;

pub struct Query;

impl Query {
    pub async fn get_user_goals<C: ConnectionTrait>(db: &C, user_id: Uuid) -> Result<Vec<GoalModel>, DbErr> {
        let goals = Goal::find()
            .filter(hikari_entity::planner::planner_goal::Column::UserId.eq(user_id))
            .order_by_desc(hikari_entity::planner::planner_goal::Column::CreatedAt)
            .all(db)
            .await;

        goals.inspect_err(|error| {
            tracing::error!(error = %error, "failed to load user goals");
        })
    }

    pub async fn get_user_goal<C: ConnectionTrait>(
        db: &C,
        user_id: Uuid,
        id: Uuid,
    ) -> Result<Option<GoalModel>, DbErr> {
        let goal = Goal::find_by_id(id)
            .filter(hikari_entity::planner::planner_goal::Column::UserId.eq(user_id))
            .one(db)
            .await;

        goal.inspect_err(|error| {
            tracing::error!(error = %error, "failed to load user goal");
        })
    }
}
