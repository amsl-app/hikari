use crate::assessment::answer;
use crate::assessment::answer::QuestionAnswer;
use crate::util::FlattenTransactionResultExt;
use hikari_entity::assessment::session::{
    ActiveModel as ActiveAssessmentSession, AssessmentStatus, Entity as AssessmentSessionEntity,
    Model as AssessmentSession,
};
use sea_orm::{ActiveModelTrait, ActiveValue, ConnectionTrait, DbErr, EntityTrait, IntoActiveValue, TransactionTrait};
use std::error::Error;
use uuid::Uuid;

pub struct Mutation;

impl Mutation {
    pub async fn new_assessment<C: ConnectionTrait>(
        conn: &C,
        user_id: Uuid,
        assessment: String,
    ) -> Result<AssessmentSession, DbErr> {
        let assessment_session = ActiveAssessmentSession {
            id: ActiveValue::Set(Uuid::new_v4()),
            assessment: ActiveValue::Set(assessment.clone()),
            user_id: ActiveValue::Set(user_id),
            status: ActiveValue::Set(AssessmentStatus::Running),
            completed: ActiveValue::NotSet,
        };

        assessment_session.insert(conn).await.inspect_err(
            |error| tracing::error!(error = error as &dyn Error, %user_id, %assessment, "failed to create assessment session")
        )
    }

    pub async fn set_assessment_status<C: ConnectionTrait>(
        conn: &C,
        id: Uuid,
        status: AssessmentStatus,
    ) -> Result<(), DbErr> {
        let completed = (status == AssessmentStatus::Finished).then_some(chrono::Local::now().naive_local());
        let assessment_session = ActiveAssessmentSession {
            id: ActiveValue::Set(id),
            assessment: ActiveValue::NotSet,
            user_id: ActiveValue::NotSet,
            status: ActiveValue::Set(status),
            completed: completed.into_active_value(),
        };

        AssessmentSessionEntity::update(assessment_session).exec(conn).await?;
        Ok(())
    }

    pub async fn finish_assessment<C: ConnectionTrait + TransactionTrait>(
        conn: &C,
        session_id: Uuid,
        answers: Vec<QuestionAnswer>,
    ) -> Result<(), DbErr> {
        conn.transaction(|conn| {
            Box::pin(async move {
                Self::set_assessment_status(conn, session_id, AssessmentStatus::Finished).await?;
                answer::Mutation::insert_or_update_many(conn, session_id, answers).await?;
                Ok(())
            })
        })
        .await
        .flatten_res()
    }
}
