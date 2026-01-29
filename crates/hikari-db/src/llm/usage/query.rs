use hikari_entity::llm::usage;
use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter};
use std::error::Error;
use uuid::Uuid;
pub struct Query;

impl Query {
    pub async fn get_usage(db: &DatabaseConnection, user_id: &Uuid) -> Result<u64, DbErr> {
        let usages = usage::Entity::find()
            .filter(usage::Column::UserId.eq(*user_id))
            .all(db)
            .await
            .inspect_err(|error| {
                tracing::error!(error = error as &dyn Error, "failed to load slots");
            })?;
        let usage = usages.iter().map(|usage| u64::from(usage.tokens)).sum();
        Ok(usage)
    }
}
