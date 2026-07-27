use hikari_entity::planner::planner_goal::{ActiveModel, Model as GoalModel};
use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, NotSet, QueryFilter};
use uuid::Uuid;

pub struct Mutation;

impl Mutation {
    pub async fn create_goal<C: ConnectionTrait>(
        db: &C,
        user_id: Uuid,
        name: String,
        description: Option<String>,
    ) -> Result<GoalModel, DbErr> {
        let goal = ActiveModel {
            id: ActiveValue::Set(Uuid::new_v4()),
            user_id: ActiveValue::Set(user_id),
            name: ActiveValue::Set(name),
            description: ActiveValue::Set(description),
            fulfilled: ActiveValue::Set(false),
            created_at: NotSet,
            updated_at: NotSet,
        };

        let res = goal.insert(db).await;
        res.inspect_err(|error| {
            tracing::error!(error = %error, "failed to create goal");
        })
    }

    pub async fn update_goal<C: ConnectionTrait>(db: &C, mut active_model: ActiveModel) -> Result<GoalModel, DbErr> {
        active_model.updated_at = ActiveValue::Set(chrono::Utc::now().naive_utc());
        let res = active_model.update(db).await;
        res.inspect_err(|error| {
            tracing::error!(error = %error, "failed to update goal");
        })
    }

    pub async fn delete_goal<C: ConnectionTrait>(db: &C, user_id: Uuid, id: Uuid) -> Result<u64, DbErr> {
        let res = hikari_entity::planner::planner_goal::Entity::delete_many()
            .filter(hikari_entity::planner::planner_goal::Column::Id.eq(id))
            .filter(hikari_entity::planner::planner_goal::Column::UserId.eq(user_id))
            .exec(db)
            .await;

        match res {
            Ok(delete_res) => Ok(delete_res.rows_affected),
            Err(error) => {
                tracing::error!(error = %error, "failed to delete goal");
                Err(error)
            }
        }
    }
}
