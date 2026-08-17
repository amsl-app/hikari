use hikari_entity::assessment::answer;
use hikari_entity::assessment::answer::{Entity as AnswerEntity, Model as Answer};
use hikari_entity::assessment::session;
use hikari_entity::assessment::session::{Entity as SessionEntity, Model as Session};
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

    pub async fn load_answers_for_assessment<C: ConnectionTrait>(
        conn: &C,
        assessment_id: &str,
        user_id: Uuid,
    ) -> Result<Vec<(Session, Vec<Answer>)>, DbErr> {
        // Left join
        let res = SessionEntity::find()
            .filter(session::Column::Assessment.eq(assessment_id))
            .filter(session::Column::UserId.eq(user_id))
            .find_with_related(AnswerEntity)
            .all(conn)
            .await
            .inspect_err(|error| {
                tracing::error!(
                    error = error as &dyn std::error::Error,
                    %assessment_id,
                    %user_id,
                    "failed to load answers for assessment"
                )
            })?;

        Ok(res)
    }

    pub async fn load_answers_for_session<C: ConnectionTrait>(
        conn: &C,
        session_id: Uuid,
        user_id: Uuid,
    ) -> Result<(Session, Vec<Answer>), DbErr> {
        // Left join
        let res = SessionEntity::find_by_id(session_id)
            .filter(session::Column::UserId.eq(user_id))
            .filter(session::Column::Id.eq(session_id))
            .find_with_related(AnswerEntity)
            .all(conn)
            .await
            .inspect_err(|error| {
                tracing::error!(
                    error = error as &dyn std::error::Error,
                    %session_id,
                    %user_id,
                    "failed to load answers for session"
                )
            })?
            .into_iter()
            .next()
            .ok_or_else(|| {
                tracing::error!(%session_id, %user_id, "no session found for user");
                DbErr::RecordNotFound(format!("No session found for user {user_id} and session {session_id}"))
            })?;

        Ok(res)
    }

    pub async fn load_answers_for_sessions<C: ConnectionTrait>(
        conn: &C,
        user_id: Uuid,
    ) -> Result<Vec<(Session, Vec<Answer>)>, DbErr> {
        // Left join
        let res = SessionEntity::find()
            .filter(session::Column::UserId.eq(user_id))
            .find_with_related(AnswerEntity)
            .all(conn)
            .await
            .inspect_err(|error| {
                tracing::error!(
                    error = error as &dyn std::error::Error,
                    %user_id,
                    "failed to load answers for sessions"
                )
            })?;

        Ok(res)
    }
}
