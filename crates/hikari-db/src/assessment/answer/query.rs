use hikari_entity::assessment::answer;
use hikari_entity::assessment::answer::{Entity as AnswerEntity, Model as Answer};
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter};
use uuid::Uuid;

pub struct Query;

impl Query {
    pub async fn load_answers<C: ConnectionTrait>(conn: &C, session_id: Uuid) -> Result<Vec<Answer>, DbErr> {
        AnswerEntity::find()
            .filter(answer::Column::AssessmentSessionId.eq(session_id))
            .all(conn)
            .await
            .inspect_err(
                |error| tracing::error!(error = error as &dyn std::error::Error, %session_id, "failed to load answers"),
            )
    }
}
