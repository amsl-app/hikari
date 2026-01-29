use hikari_entity::history;
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter};
use uuid::Uuid;

use hikari_entity::history::history_module::{Entity as HistoryModuleEntity, Model as HistoryModule};
use hikari_entity::history::{Entity as HistoryEntity, Model as History};

pub struct Query;

impl Query {
    pub async fn get_for_user<C: ConnectionTrait>(
        conn: &C,
        user_id: Uuid,
    ) -> Result<Vec<(History, HistoryModule)>, DbErr> {
        let history = HistoryEntity::find()
            .filter(history::Column::UserId.eq(user_id))
            .inner_join(HistoryModuleEntity)
            .select_also(HistoryModuleEntity)
            .all(conn)
            .await?;
        history
            .into_iter()
            .map(|(history, module)| {
                let Some(module) = module else {
                    // this should never happen
                    tracing::error!(id = %history.id.as_hyphenated(), "empty history entry");
                    return Err(DbErr::RecordNotFound(
                        "history module query returned empty result".to_owned(),
                    ));
                };
                Ok((history, module))
            })
            .collect()
    }
}
