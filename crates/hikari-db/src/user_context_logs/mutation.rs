use hikari_entity::user_context_logs::{ActiveModel, Entity};
use sea_orm::prelude::Json;
use sea_orm::{ActiveValue, ConnectionTrait, DbErr, EntityTrait, InsertResult};
use uuid::Uuid;

pub struct Mutation;

impl Mutation {
    pub async fn insert<C: ConnectionTrait>(
        conn: &C,
        user_id: Uuid,
        log_type: String,
        data: Json,
    ) -> Result<InsertResult<ActiveModel>, DbErr> {
        Entity::insert(ActiveModel {
            user_id: ActiveValue::Set(user_id),
            created_at: ActiveValue::Set(chrono::Utc::now().naive_utc()),
            r#type: ActiveValue::Set(log_type),
            data: ActiveValue::Set(data),
        })
        .exec(conn)
        .await
    }
}
