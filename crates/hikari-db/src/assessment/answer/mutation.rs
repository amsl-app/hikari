use hikari_entity::assessment::answer;
use hikari_entity::assessment::answer::{
    ActiveModel as ActiveAnswer, AnswerType, Entity as AnswerEntity, Model as Answer,
};
use sea_orm::sea_query::OnConflict;
use sea_orm::{ActiveValue, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter};
use std::error::Error;
use uuid::Uuid;

pub struct QuestionAnswer {
    pub question: String,
    pub answer_type: AnswerType,
    pub data: String,
}

pub struct Mutation;

impl Mutation {
    pub async fn insert_or_update<C: ConnectionTrait>(
        conn: &C,
        assessment_session_id: Uuid,
        question: String,
        answer_type: AnswerType,
        data: String,
    ) -> Result<Answer, DbErr> {
        let data = ActiveAnswer {
            assessment_session_id: ActiveValue::Set(assessment_session_id),
            question: ActiveValue::Set(question.clone()),
            answer_type: ActiveValue::Set(answer_type),
            data: ActiveValue::Set(data),
        };

        let mut on_conflict = OnConflict::columns([answer::Column::AssessmentSessionId, answer::Column::Question]);
        on_conflict.update_columns([answer::Column::AnswerType, answer::Column::Data]);
        AnswerEntity::insert(data)
            .on_conflict(on_conflict)
            .do_nothing()
            .exec(conn)
            .await.inspect_err(
                |error| tracing::error!(error = error as &dyn Error, %assessment_session_id, %question, "failed to insert or update answer")
        )?;

        let res = AnswerEntity::find()
            .filter(answer::Column::AssessmentSessionId.eq(assessment_session_id))
            .filter(answer::Column::Question.eq(question.clone()))
            .one(conn)
            .await.inspect_err(
                |error| tracing::error!(error = error as &dyn Error, %assessment_session_id, %question, "failed to load answer after insertion")
        )?;

        res.ok_or_else(|| {
            tracing::error!(%assessment_session_id, %question, "answer not found after insertion");
            DbErr::RecordNotFound("answer not found after insertion".to_owned())
        })
    }

    pub async fn insert_or_update_many<C: ConnectionTrait>(
        conn: &C,
        assessment_session_id: Uuid,
        question_answers: Vec<QuestionAnswer>,
    ) -> Result<(), DbErr> {
        if question_answers.is_empty() {
            return Ok(());
        }
        let data: Vec<_> = question_answers
            .into_iter()
            .map(|qa| ActiveAnswer {
                assessment_session_id: ActiveValue::Set(assessment_session_id),
                question: ActiveValue::Set(qa.question),
                answer_type: ActiveValue::Set(qa.answer_type),
                data: ActiveValue::Set(qa.data),
            })
            .collect();

        let mut on_conflict = OnConflict::columns([answer::Column::AssessmentSessionId, answer::Column::Question]);
        on_conflict.update_columns([answer::Column::AnswerType, answer::Column::Data]);
        AnswerEntity::insert_many(data)
            .on_conflict(on_conflict)
            .do_nothing()
            .exec(conn)
            .await?;
        Ok(())
    }
}
