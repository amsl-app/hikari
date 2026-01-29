use hikari_entity::user_handle;
use hikari_entity::user_handle::{Entity, Model};
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter};
use uuid::Uuid;

pub struct Query;

impl Query {
    pub async fn get_for_user<C: ConnectionTrait>(db: &C, user_id: Uuid) -> Result<Vec<Model>, DbErr> {
        Entity::find()
            .filter(user_handle::Column::UserId.eq(user_id))
            .all(db)
            .await
    }

    pub async fn get_max_handle_length<C: ConnectionTrait>(db: &C) -> Result<Option<usize>, DbErr> {
        let res = Entity::find().all(db).await;

        match res {
            Err(error) => {
                tracing::error!(
                    error = &error as &dyn std::error::Error,
                    "failed to load max handle length"
                );
                Err(error)
            }
            Ok(entries) => {
                if entries.is_empty() {
                    return Ok(None);
                }
                let max = entries
                    .iter()
                    .map(|row| row.handle.len())
                    .max()
                    .ok_or_else(|| DbErr::Type("failed to find max handle length".to_string()))?;
                Ok(Some(max))
            }
        }
    }
}
