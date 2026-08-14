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

    pub async fn load_answes_for_assessment<C: ConnectionTrait>(
        conn: &C,
        assessment_id: &str,
        user_id: Uuid,
    ) -> Result<Vec<(Session, Vec<Answer>)>, DbErr> {
        // Left join
        let res = SessionEntity::find()
            .filter(session::Column::Assessment.eq(assessment_id))
            .filter(session::Column::UserId.eq(user_id))
            .find_also_related(AnswerEntity)
            .all(conn)
            .await
            .inspect_err(|error| {
                tracing::error!(
                    error = error as &dyn std::error::Error,
                    %assessment_id,
                    %user_id,
                    "failed to load answers for assessment"
                )
            })?
            .into_iter();

        // Aggregate based on session
        let mut sessions_with_answers: Vec<(Session, Vec<Answer>)> = Vec::new();
        for (session, answer) in res {
            if let Some(answer) = answer {
                if let Some((_, answers)) = sessions_with_answers.iter_mut().find(|(s, _)| s.id == session.id) {
                    answers.push(answer);
                } else {
                    sessions_with_answers.push((session, vec![answer]));
                }
            } else {
                if !sessions_with_answers.iter().any(|(s, _)| s.id == session.id) {
                    sessions_with_answers.push((session, vec![]));
                }
            }
        }

        Ok(sessions_with_answers)
    }
}
