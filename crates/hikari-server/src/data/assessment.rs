use crate::routes::api::v0::assessment::error::Error;
use hikari_config::assessment::AssessmentConfig;
use hikari_config::assessment::question::{AnswerValue, AssessmentQuestion, QuestionBody};
use hikari_db::assessment::answer::QuestionAnswer;
use hikari_db::util::FlattenTransactionResultExt;
use hikari_entity::assessment::session::Model as AssessmentSession;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, TransactionTrait};
use serde_derive::Deserialize;
use std::collections::HashMap;
use utoipa::ToSchema;
use uuid::Uuid;

pub(crate) trait HasSeaOrmAnswerType {
    fn sea_orm_answer_type(&self) -> hikari_entity::assessment::answer::AnswerType;
}

impl HasSeaOrmAnswerType for AssessmentQuestion {
    fn sea_orm_answer_type(&self) -> hikari_entity::assessment::answer::AnswerType {
        match self.body {
            QuestionBody::Scale(_) => hikari_entity::assessment::answer::AnswerType::Int,
            QuestionBody::Textfield(_) | QuestionBody::Textarea(_) | QuestionBody::MultiChoice(_) => {
                hikari_entity::assessment::answer::AnswerType::Text
            }
            QuestionBody::Select(_) | QuestionBody::SingleChoice(_) => {
                hikari_entity::assessment::answer::AnswerType::Bool
            }
        }
    }
}

pub(crate) fn answer_value_to_string(val: AnswerValue) -> String {
    match val {
        AnswerValue::Bool { value } => value.to_string(),
        AnswerValue::Text { value } => value,
        AnswerValue::SmallInt { value } => value.to_string(),
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(tag = "type")]
#[schema(example = json!({"question_id": "some-id", "value": true}))]
pub(crate) struct AnswerRequest {
    pub question_id: String,

    #[serde(flatten)]
    pub answer: AnswerValue,
}

pub(crate) fn build_assessment_answers_sea_orm(
    assessment: &str,
    config: &AssessmentConfig,
    answers: Vec<AnswerRequest>,
) -> Result<Vec<QuestionAnswer>, Error> {
    let assessment = config.get(assessment).ok_or(Error::AssessmentConfigNotFound)?;

    let mut answers = answers
        .into_iter()
        .map(|a| (a.question_id.clone(), a))
        .collect::<HashMap<_, _>>();
    assessment
        .questions
        .values()
        .map(|question| {
            Ok(QuestionAnswer {
                question: question.id.clone(),
                answer_type: question.sea_orm_answer_type(),
                data: answer_value_to_string(
                    answers
                        .remove(question.id.as_str())
                        .ok_or(Error::MissingAnswer(question.id.clone()))?
                        .answer,
                ),
            })
        })
        .collect::<Result<Vec<_>, Error>>()
}

pub(crate) async fn get_or_start_session<C: ConnectionTrait>(
    conn: &C,
    user_id: Uuid,
    assessment_id: &str,
) -> Result<AssessmentSession, DbErr> {
    if let Some(session) =
        hikari_db::assessment::session::Query::load_running_session(conn, assessment_id, user_id).await?
    {
        return Ok(session);
    }

    let session =
        hikari_db::assessment::session::Mutation::new_assessment(conn, user_id, assessment_id.to_owned()).await?;
    tracing::debug!(
        user_id = %user_id.as_hyphenated(),
        session_id = %session.id.as_hyphenated(),
        "started assessment session"
    );
    Ok(session)
}

pub(crate) async fn require_running_session<C: ConnectionTrait>(
    conn: &C,
    user_id: Uuid,
    assessment_id: &str,
) -> Result<AssessmentSession, Error> {
    hikari_db::assessment::session::Query::load_running_session(conn, assessment_id, user_id)
        .await?
        .ok_or(Error::NotRunning)
}

#[tracing::instrument(skip(conn, session, config, answers), fields(user_id = %user_id.as_hyphenated(), assessment_id = %session.assessment.as_str(), session_id = %session.id.as_hyphenated()))]
pub(crate) async fn submit_session(
    conn: &DatabaseConnection,
    user_id: Uuid,
    session: AssessmentSession,
    config: &AssessmentConfig,
    answers: Vec<AnswerRequest>,
) -> Result<(), Error> {
    let question_answers = build_assessment_answers_sea_orm(&session.assessment, config, answers)?;
    let session_id = session.id;

    conn.transaction(|txn| {
        Box::pin(async move {
            hikari_db::assessment::session::Mutation::finish_assessment(txn, session_id, question_answers).await?;
            hikari_db::history::history_assessment::Mutation::create(txn, user_id, session_id).await
        })
    })
    .await
    .flatten_res()
    .inspect_err(|e| {
        tracing::error!(
            user_id = %user_id.as_hyphenated(),
            session_id = %session_id.as_hyphenated(),
            "failed to submit assessment session: {e:?}"
        );
    })?;
    Ok(())
}
