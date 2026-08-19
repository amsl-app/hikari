use hikari_entity::planner::{planner_goal, planner_goal_milestone, planner_milestone};

use crate::util::FlattenTransactionResultExt;
use sea_orm::{ActiveValue, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, TransactionTrait};

use uuid::Uuid;

pub struct Mutation;

impl Mutation {
    pub async fn set_milestone_goals<C: ConnectionTrait + TransactionTrait>(
        db: &C,
        user_id: Uuid,
        milestone_id: Uuid,
        goal_ids: Vec<Uuid>,
    ) -> Result<Vec<planner_goal::Model>, DbErr> {
        let count = goal_ids.len();

        let goals = planner_goal::Entity::find()
            .filter(planner_goal::Column::UserId.eq(user_id))
            .filter(planner_goal::Column::Id.is_in(goal_ids))
            .all(db)
            .await?;

        if count != goals.len() {
            tracing::error!(user_id = %user_id, milestone_id = %milestone_id, "did not find the correct number of goals");
            return Err(DbErr::RecordNotFound("goals".to_string()));
        }

        let milestone = planner_milestone::Entity::find_by_id(milestone_id)
            .filter(planner_milestone::Column::UserId.eq(user_id))
            .one(db)
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("failed to find milestone".to_string()))?;

        let res = db
            .transaction::<_, Vec<planner_goal::Model>, DbErr>(|txn| {
                Box::pin(async move {
                    planner_goal_milestone::Entity::delete_many()
                        .filter(planner_goal_milestone::Column::MilestoneId.eq(milestone.id))
                        .exec(txn)
                        .await?;
                    if !goals.is_empty() {
                        planner_goal_milestone::Entity::insert_many(goals.iter().map(|model| {
                            planner_goal_milestone::ActiveModel {
                                goal_id: ActiveValue::Set(model.id),
                                milestone_id: ActiveValue::Set(milestone.id),
                            }
                        }))
                        .exec(txn)
                        .await?;
                    }
                    Ok(goals)
                })
            })
            .await
            .flatten_res();

        res.inspect_err(|error| tracing::error!(error = %error, "failed to set milestone goals"))
    }
}
