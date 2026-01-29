use base64::Engine;
use chrono::NaiveDateTime;
use hikari_entity::journal::journal_summary;
use hikari_entity::journal::journal_summary::{Entity, Model};
use hikari_entity::journal::journal_topic;
use hikari_entity::journal::journal_topic::{Entity as JournalTopicEntity, Model as JournalTopicModel};
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QueryOrder};
use std::error::Error;
use uuid::Uuid;

pub struct Query;

impl Query {
    pub async fn find<C: ConnectionTrait>(
        conn: &C,
        user_id: Uuid,
        key: &[u8; 32],
        from_date: NaiveDateTime,
        to_date: NaiveDateTime,
    ) -> Result<Option<(Model, Vec<JournalTopicModel>)>, DbErr> {
        let hex_key = base64::engine::general_purpose::STANDARD.encode(key);
        tracing::debug!(%user_id, key = hex_key, %from_date, %to_date, "loading user journal summary");
        let summary = match Entity::find()
            .filter(journal_summary::Column::UserId.eq(user_id))
            .filter(journal_summary::Column::Key.eq(key.as_slice()))
            .filter(journal_summary::Column::CreatedAt.gte(from_date))
            .filter(journal_summary::Column::CreatedAt.lt(to_date))
            .order_by_desc(journal_summary::Column::CreatedAt)
            .one(conn)
            .await
        {
            Ok(res) => res,
            Err(error) => {
                tracing::error!(
                    error = &error as &dyn Error,
                    key = hex_key,
                    "error loading journal summary"
                );
                return Err(error);
            }
        };

        let Some(summary) = summary else {
            tracing::debug!(key = hex_key, "could not find journal summary");
            return Ok(None);
        };

        let topics = JournalTopicEntity::find()
            .filter(journal_topic::Column::JournalSummaryId.eq(summary.id))
            .all(conn)
            .await?;

        Ok(Some((summary, topics)))
    }
}
