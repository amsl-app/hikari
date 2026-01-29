use hikari_entity::history;
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter};
use uuid::Uuid;

use hikari_entity::history::history_session::{Entity as HistorySessionEntity, Model as HistorySession};
use hikari_entity::history::{Entity as HistoryEntity, Model as History};

pub struct Query;

impl Query {
    pub async fn get_for_user<C: ConnectionTrait>(
        conn: &C,
        user_id: Uuid,
    ) -> Result<Vec<(History, HistorySession)>, DbErr> {
        let history = HistoryEntity::find()
            .filter(history::Column::UserId.eq(user_id))
            .inner_join(HistorySessionEntity)
            .select_also(HistorySessionEntity)
            .all(conn)
            .await?;
        history
            .into_iter()
            .map(|(history, module)| {
                let module = module.ok_or_else(|| {
                    tracing::error!(id = %history.id.as_hyphenated(), "empty history entry");
                    DbErr::RecordNotFound("history session query returned empty result".to_owned())
                })?;
                Ok((history, module))
            })
            .collect()
    }
}
