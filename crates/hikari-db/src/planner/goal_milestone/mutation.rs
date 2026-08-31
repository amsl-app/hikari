use hikari_entity::planner::planner_goal_milestone;
use sea_orm::{ActiveValue, ConnectionTrait, DbErr, EntityTrait};
use uuid::Uuid;

pub struct Mutation {}

impl Mutation {
    pub async fn insert_goal_links<C: ConnectionTrait>(
        db: &C,
        milestone_id: Uuid,
        goals: Vec<Uuid>,
    ) -> Result<(), DbErr> {
        if goals.is_empty() {
            return Ok(());
        }
        planner_goal_milestone::Entity::insert_many(goals.iter().map(|goal| planner_goal_milestone::ActiveModel {
            goal_id: ActiveValue::Set(*goal),
            milestone_id: ActiveValue::Set(milestone_id),
        }))
        .exec(db)
        .await?;
        Ok(())
    }
}
