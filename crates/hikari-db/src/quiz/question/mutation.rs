use hikari_entity::quiz::question::{self, BloomLevel};
use sea_orm::{ActiveModelTrait, ActiveValue::NotSet, DatabaseConnection, DbErr, Set};
use uuid::Uuid;
pub struct Mutation;

impl Mutation {
    #[allow(clippy::too_many_arguments)]
    pub async fn create_text_question(
        db: &DatabaseConnection,
        quiz_id: &Uuid,
        question: &str,
        ai_solution: &str,
        level: &BloomLevel,
        session_id: &str,
        topic: &str,
        content: &str,
    ) -> Result<question::Model, DbErr> {
        Self::create_question(
            db,
            quiz_id,
            question,
            Some(ai_solution),
            &question::QuestionType::Text,
            None,
            level,
            session_id,
            topic,
            content,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_multiple_choice_question(
        db: &DatabaseConnection,
        quiz_id: &Uuid,
        question: &str,
        options: &str,
        level: &BloomLevel,
        session_id: &str,
        topic: &str,
        content: &str,
    ) -> Result<question::Model, DbErr> {
        Self::create_question(
            db,
            quiz_id,
            question,
            None,
            &question::QuestionType::MultipleChoice,
            Some(options),
            level,
            session_id,
            topic,
            content,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_question(
        db: &DatabaseConnection,
        quiz_id: &Uuid,
        question: &str,
        ai_solution: Option<&str>,
        question_type: &question::QuestionType,
        options: Option<&str>,
        level: &BloomLevel,
        session_id: &str,
        topic: &str,
        content: &str,
    ) -> Result<question::Model, DbErr> {
        let quiz = question::ActiveModel {
            id: Set(Uuid::new_v4()),
            quiz_id: Set(*quiz_id),
            question: Set(question.to_string()),
            level: Set(*level),
            session_id: Set(session_id.to_string()),
            topic: Set(topic.to_string()),
            content: Set(content.to_string()),
            r#type: Set(*question_type),
            options: Set(options.map(std::string::ToString::to_string)),
            created_at: Set(chrono::Utc::now().naive_utc()),
            answered_at: NotSet,
            answer: NotSet,
            evaluation: NotSet,
            grade: NotSet,
            ai_solution: Set(ai_solution.map(std::string::ToString::to_string)),
            status: Set(question::Status::Open),
            feedback: NotSet,
            feedback_explanation: NotSet,
        };
        quiz.insert(db).await
    }

    pub async fn add_evaluation(
        db: &DatabaseConnection,
        question_id: &Uuid,
        answer: &str,
        evaluation: &str,
        grading: &i32,
    ) -> Result<question::Model, DbErr> {
        let question = super::Query::get_question_by_id(db, question_id)
            .await?
            .ok_or_else(|| DbErr::Custom("question not found".to_string()))?;

        let mut question: question::ActiveModel = question.into();
        question.evaluation = Set(Some(evaluation.to_string()));
        question.grade = Set(Some(*grading));
        question.answer = Set(Some(answer.to_string()));
        question.answered_at = Set(Some(chrono::Utc::now().naive_utc()));
        question.status = Set(question::Status::Finished);
        question.update(db).await
    }

    pub async fn skip_question(db: &DatabaseConnection, question_id: &Uuid) -> Result<question::Model, DbErr> {
        let question = super::Query::get_question_by_id(db, question_id)
            .await?
            .ok_or_else(|| DbErr::Custom("question not found".to_string()))?;

        let mut question: question::ActiveModel = question.into();
        question.status = Set(question::Status::Skipped);
        question.update(db).await
    }

    pub async fn add_feedback(
        db: &DatabaseConnection,
        question_id: &Uuid,
        feedback: &question::Feedback,
        feedback_explanation: Option<&str>,
    ) -> Result<question::Model, DbErr> {
        let question = super::Query::get_question_by_id(db, question_id)
            .await?
            .ok_or_else(|| DbErr::Custom("question not found".to_string()))?;

        let mut question: question::ActiveModel = question.into();
        question.feedback = Set(Some(feedback.clone()));
        question.feedback_explanation = Set(feedback_explanation.map(std::string::ToString::to_string));
        question.update(db).await
    }
}
