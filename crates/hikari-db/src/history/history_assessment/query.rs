use hikari_entity::history;
use hikari_entity::history::history_assessment::{Entity as HistoryAssessmentEntity, Model as HistoryAssessment};
use hikari_entity::history::{Entity as HistoryEntity, Model as History};
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter};
use uuid::Uuid;

pub struct Query;

impl Query {
    pub async fn get_for_user<C: ConnectionTrait>(
        conn: &C,
        user_id: Uuid,
    ) -> Result<Vec<(History, HistoryAssessment)>, DbErr> {
        let history = HistoryEntity::find()
            .filter(history::Column::UserId.eq(user_id))
            .inner_join(HistoryAssessmentEntity)
            .select_also(HistoryAssessmentEntity)
            .all(conn)
            .await?;
        history
            .into_iter()
            .map(|(history, module)| {
                let Some(module) = module else {
                    // this should never happen
                    tracing::error!(id = %history.id.as_hyphenated(), "empty history entry");
                    return Err(DbErr::RecordNotFound(
                        "history assessment query returned empty result".to_owned(),
                    ));
                };
                Ok((history, module))
            })
            .collect()
    }
}
