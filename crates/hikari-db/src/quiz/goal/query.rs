use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QuerySelect, JoinType};
use std::error::Error;
use uuid::Uuid;
use hikari_entity::quiz::goal::{Entity as Goal, Model as GoalModel};
pub struct Query;

impl Query {
    pub async fn get_goals_by_user_and_module(
        db: &DatabaseConnection,
        user_id: &Uuid,
        module_id: &str,
    ) -> Result<Vec<GoalModel>, DbErr> {
        let query = Goal::find()
            .filter(
                <hikari_entity::quiz::goal::Entity as sea_orm::EntityTrait>::Column::UserId
                    .eq(*user_id),
            )
            .filter(
                <hikari_entity::quiz::goal::Entity as sea_orm::EntityTrait>::Column::ModuleId
                    .eq(module_id),
            );

        query.all(db).await.inspect_err(|error| {
            tracing::error!(
                error = error as &dyn Error,
                user_id = %user_id,
                module_id = %module_id,
                "failed to load goals by user and module"
            );
        })
    }

    pub async fn get_goal_by_id_and_user(
        db: &DatabaseConnection,
        goal_id: &Uuid,
        user_id: &Uuid,
    ) -> Result<Option<GoalModel>, DbErr> {

        Goal::find_by_id(*goal_id)
            .filter(
                <hikari_entity::quiz::goal::Entity as sea_orm::EntityTrait>::Column::UserId.eq(*user_id)
            )
            .one(db)
            .await
    }
}