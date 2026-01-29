use chrono::Utc;
use hikari_entity::llm::usage;
use sea_orm::{DatabaseConnection, DbErr, EntityTrait, InsertResult, IntoActiveValue};
use uuid::Uuid;

pub struct Mutation;

impl Mutation {
    pub async fn add_usage(
        conn: &DatabaseConnection,
        user_id: &Uuid,
        tokens: u32,
        step: String,
    ) -> Result<InsertResult<usage::ActiveModel>, DbErr> {
        let model = usage::ActiveModel {
            user_id: user_id.into_active_value(),
            tokens: tokens.into_active_value(),
            time: Utc::now().naive_utc().into_active_value(),
            step: step.into_active_value(),
        };

        usage::Entity::insert(model).exec(conn).await
    }
}
