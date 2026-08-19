use hikari_entity::planner::planner_goal_milestone::{Column, Entity as GoalMilestone, Model};
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter};
use uuid::Uuid;

pub struct Query;

impl Query {
    pub async fn get_links_for_goals<C: ConnectionTrait>(db: &C, goal_ids: &[Uuid]) -> Result<Vec<Model>, DbErr> {
        if goal_ids.is_empty() {
            return Ok(vec![]);
        }
        GoalMilestone::find()
            .filter(Column::GoalId.is_in(goal_ids.to_vec()))
            .all(db)
            .await
            .inspect_err(|error| tracing::error!(error = %error, "failed to load goal-milestone links for goals"))
    }

    pub async fn get_links_for_milestones<C: ConnectionTrait>(
        db: &C,
        milestone_ids: &[Uuid],
    ) -> Result<Vec<Model>, DbErr> {
        if milestone_ids.is_empty() {
            return Ok(vec![]);
        }
        GoalMilestone::find()
            .filter(Column::MilestoneId.is_in(milestone_ids.to_vec()))
            .all(db)
            .await
            .inspect_err(|error| tracing::error!(error = %error, "failed to load goal-milestone links for milestones"))
    }
}
