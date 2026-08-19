use chrono::NaiveDate;
use hikari_entity::planner::{
    planner_goal, planner_goal_milestone,
    planner_milestone::{ActiveModel, Column, Entity as PlannerMilestone, Model as PlannerMilestoneModel},
};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, NotSet, QueryFilter,
    TransactionTrait, sea_query,
};
use uuid::Uuid;

use crate::util::FlattenTransactionResultExt;

pub struct MilestoneInput {
    pub title: String,
    pub date: NaiveDate,
    pub description: Option<String>,
    pub module_id: Option<String>,
    pub origin_id: Option<String>,
    pub goals: Vec<Uuid>,
}

pub struct Mutation;

impl Mutation {
    /// Fetches the goals owned by `user_id` for `goal_ids`, erroring if any id doesn't resolve.
    async fn require_owned_goals<C: ConnectionTrait>(
        db: &C,
        user_id: Uuid,
        goal_ids: Vec<Uuid>,
    ) -> Result<Vec<planner_goal::Model>, DbErr> {
        if goal_ids.is_empty() {
            return Ok(vec![]);
        }
        let count = goal_ids.len();
        let goals = planner_goal::Entity::find()
            .filter(planner_goal::Column::UserId.eq(user_id))
            .filter(planner_goal::Column::Id.is_in(goal_ids))
            .all(db)
            .await?;
        if goals.len() != count {
            tracing::error!(user_id = %user_id, "did not find the correct number of goals");
            return Err(DbErr::RecordNotFound("goals".to_string()));
        }
        Ok(goals)
    }

    pub async fn create_milestone<C: ConnectionTrait + TransactionTrait>(
        db: &C,
        user_id: Uuid,
        input: MilestoneInput,
    ) -> Result<PlannerMilestoneModel, DbErr> {
        let goals = Self::require_owned_goals(db, user_id, input.goals).await?;

        let model = ActiveModel {
            id: ActiveValue::Set(Uuid::new_v4()),
            user_id: ActiveValue::Set(user_id),
            title: ActiveValue::Set(input.title),
            date: ActiveValue::Set(input.date),
            description: ActiveValue::Set(input.description),
            module_id: ActiveValue::Set(input.module_id),
            origin_id: ActiveValue::Set(input.origin_id),
            created_at: NotSet,
            updated_at: NotSet,
        };

        db.transaction::<_, PlannerMilestoneModel, DbErr>(|txn| {
            Box::pin(async move {
                let created = model.insert(txn).await?;
                if !goals.is_empty() {
                    planner_goal_milestone::Entity::insert_many(goals.iter().map(|goal| {
                        planner_goal_milestone::ActiveModel {
                            goal_id: ActiveValue::Set(goal.id),
                            milestone_id: ActiveValue::Set(created.id),
                        }
                    }))
                    .exec(txn)
                    .await?;
                }
                Ok(created)
            })
        })
        .await
        .flatten_res()
        .inspect_err(|error| tracing::error!(%error, "failed to create milestone"))
    }

    pub async fn update_milestone<C: ConnectionTrait + TransactionTrait>(
        db: &C,
        mut active_model: ActiveModel,
        goals: Option<Vec<Uuid>>,
    ) -> Result<PlannerMilestoneModel, DbErr> {
        active_model.updated_at = ActiveValue::Set(chrono::Utc::now().naive_utc());

        let milestone_id = *active_model.id.try_as_ref().expect("milestone id must be set");
        let user_id = *active_model.user_id.try_as_ref().expect("user id must be set");

        let goals = match goals {
            Some(goal_ids) => Some(Self::require_owned_goals(db, user_id, goal_ids).await?),
            None => None,
        };

        db.transaction::<_, PlannerMilestoneModel, DbErr>(|txn| {
            Box::pin(async move {
                let updated = active_model.update(txn).await?;
                if let Some(goals) = goals {
                    planner_goal_milestone::Entity::delete_many()
                        .filter(planner_goal_milestone::Column::MilestoneId.eq(milestone_id))
                        .exec(txn)
                        .await?;
                    if !goals.is_empty() {
                        planner_goal_milestone::Entity::insert_many(goals.iter().map(|goal| {
                            planner_goal_milestone::ActiveModel {
                                goal_id: ActiveValue::Set(goal.id),
                                milestone_id: ActiveValue::Set(milestone_id),
                            }
                        }))
                        .exec(txn)
                        .await?;
                    }
                }
                Ok(updated)
            })
        })
        .await
        .flatten_res()
        .inspect_err(|error| tracing::error!(%error, "failed to update milestone"))
    }

    pub async fn delete_milestone<C: ConnectionTrait>(db: &C, user_id: Uuid, id: Uuid) -> Result<u64, DbErr> {
        let res = PlannerMilestone::delete_many()
            .filter(Column::Id.eq(id))
            .filter(Column::UserId.eq(user_id))
            .exec(db)
            .await
            .inspect_err(|error| tracing::error!(%error, "failed to delete milestone"))?;
        Ok(res.rows_affected)
    }

    pub async fn import_milestones<C: ConnectionTrait>(
        db: &C,
        user_id: Uuid,
        inputs: Vec<MilestoneInput>,
    ) -> Result<Vec<PlannerMilestoneModel>, DbErr> {
        // Here we ignore the goals, since imported modules never have goals by default. Goals can be added later by the user.
        if inputs.is_empty() {
            return Ok(vec![]);
        }
        let ids: Vec<Uuid> = (0..inputs.len()).map(|_| Uuid::new_v4()).collect();
        let models = ids.iter().zip(inputs).map(|(id, input)| ActiveModel {
            id: ActiveValue::Set(*id),
            user_id: ActiveValue::Set(user_id),
            title: ActiveValue::Set(input.title),
            date: ActiveValue::Set(input.date),
            description: ActiveValue::Set(input.description),
            module_id: ActiveValue::Set(input.module_id),
            origin_id: ActiveValue::Set(input.origin_id),
            created_at: NotSet,
            updated_at: NotSet,
        });
        PlannerMilestone::insert_many(models)
            .on_conflict(
                sea_query::OnConflict::columns([Column::UserId, Column::ModuleId, Column::OriginId])
                    .do_nothing()
                    .to_owned(),
            )
            .exec(db)
            .await
            .inspect_err(|error| tracing::error!(%error, "failed to import milestones"))?;
        PlannerMilestone::find()
            .filter(Column::Id.is_in(ids))
            .filter(Column::UserId.eq(user_id))
            .all(db)
            .await
            .inspect_err(|error| tracing::error!(%error, "failed to fetch imported milestones"))
    }
}
