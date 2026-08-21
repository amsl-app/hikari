use sea_orm::{ActiveModelTrait, DatabaseConnection, DbErr, Set};
use uuid::Uuid;
use hikari_entity::quiz::quiz_question_attempt;
use crate::quiz::quiz_question_attempt::query::Query;

pub struct Mutation;

impl Mutation {

    pub async fn add_evaluation(
        db: &DatabaseConnection,
        quiz_id: &Uuid,
        question_id: &Uuid,
        attempt: &i32,
        answer: &str,
        evaluation: &str,
        grading: &i32,
    ) -> Result<quiz_question_attempt::Model, DbErr> {
        let quiz_question_attempt = Query::get_attempt(db, quiz_id, question_id, attempt)
            .await?
            .ok_or_else(|| DbErr::Custom("attempt not found".to_string()))?;

        let mut attempt: quiz_question_attempt::ActiveModel = quiz_question_attempt.into();
        attempt.evaluation = Set(Some(evaluation.to_string()));
        attempt.grade = Set(Some(*grading));
        attempt.answer = Set(Some(answer.to_string()));
        attempt.answered_at = Set(Some(chrono::Utc::now().naive_utc()));
        attempt.status = Set(quiz_question_attempt::Status::Finished);
        attempt.update(db).await
    }

    pub async fn skip_question(
        db: &DatabaseConnection,
        quiz_id: &Uuid,
        question_id: &Uuid,
        attempt: &i32
    ) -> Result<quiz_question_attempt::Model, DbErr> {
        let quiz_question_attempt = Query::get_attempt(db, quiz_id, question_id, attempt)
            .await?
            .ok_or_else(|| DbErr::Custom("attempt not found".to_string()))?;

        let mut attempt: quiz_question_attempt::ActiveModel = quiz_question_attempt.into();
        attempt.status = Set(quiz_question_attempt::Status::Skipped);
        attempt.update(db).await
    }

    pub async fn add_feedback(
        db: &DatabaseConnection,
        quiz_id: &Uuid,
        question_id: &Uuid,
        attempt: &i32,
        feedback: &quiz_question_attempt::Feedback,
        feedback_explanation: Option<&str>,
    ) -> Result<quiz_question_attempt::Model, DbErr> {
        let quiz_question_attempt = Query::get_attempt(db, quiz_id, question_id, attempt)
            .await?
            .ok_or_else(|| DbErr::Custom("attempt not found".to_string()))?;

        let mut attempt: quiz_question_attempt::ActiveModel = quiz_question_attempt.into();
        attempt.feedback = Set(Some(feedback.clone()));
        attempt.feedback_explanation = Set(feedback_explanation.map(ToString::to_string));
        attempt.update(db).await
    }

}
