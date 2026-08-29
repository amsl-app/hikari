use hikari_entity::planner::planner_goal::{Entity as Goal, Model as GoalModel};
use hikari_entity::planner::planner_goal_milestone;
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QueryOrder};
use std::collections::HashMap;
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

    pub async fn get_user_goals_by_ids<C: ConnectionTrait>(
        db: &C,
        user_id: Uuid,
        mut ids: Vec<Uuid>,
    ) -> Result<Vec<GoalModel>, DbErr> {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        ids.sort_unstable();
        ids.dedup();
        let len = ids.len();
        let res = Goal::find()
            .filter(hikari_entity::planner::planner_goal::Column::UserId.eq(user_id))
            .filter(hikari_entity::planner::planner_goal::Column::Id.is_in(ids))
            .all(db)
            .await
            .inspect_err(|error| tracing::error!(error = %error, "failed to load goals by ids"))?;

        if res.len() != len {
            return Err(DbErr::RecordNotFound("one or more goal ids do not exist".to_owned()));
        }

        Ok(res)
    }

    pub async fn get_goals_by_milestone_ids<C: ConnectionTrait>(
        db: &C,
        user_id: Uuid,
        milestone_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, Vec<GoalModel>>, DbErr> {
        if milestone_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let rows: Vec<(planner_goal_milestone::Model, Option<GoalModel>)> = planner_goal_milestone::Entity::find()
            .filter(planner_goal_milestone::Column::MilestoneId.is_in(milestone_ids.to_vec()))
            .find_also_related(Goal)
            .filter(hikari_entity::planner::planner_goal::Column::UserId.eq(user_id))
            .all(db)
            .await
            .inspect_err(|error| tracing::error!(error = %error, "failed to load goals for milestones"))?;

        let mut goals_by_milestone: HashMap<Uuid, Vec<GoalModel>> = HashMap::new();
        for (link, goal) in rows {
            if let Some(goal) = goal {
                goals_by_milestone.entry(link.milestone_id).or_default().push(goal);
            }
        }
        Ok(goals_by_milestone)
    }
}
