use hikari_entity::quiz::question::{self, BloomLevel};
use sea_orm::{ActiveModelTrait, ActiveValue::NotSet, DatabaseConnection, DbErr, Set};
use uuid::Uuid;
pub struct Mutation;

impl Mutation {
    #[allow(clippy::too_many_arguments)]
    pub async fn create_text_question(
        db: &DatabaseConnection,
        question: &str,
        ai_solution: &str,
        level: &BloomLevel,
        topic: &str,
        content: &str,
    ) -> Result<question::Model, DbErr> {
        Self::create_question(
            db,
            question,
            Some(ai_solution),
            &question::QuestionType::Text,
            None,
            level,
            topic,
            content,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_multiple_choice_question(
        db: &DatabaseConnection,
        question: &str,
        options: &str,
        level: &BloomLevel,
        topic: &str,
        content: &str,
    ) -> Result<question::Model, DbErr> {
        Self::create_question(
            db,
            question,
            None,
            &question::QuestionType::MultipleChoice,
            Some(options),
            level,
            topic,
            content,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_question(
        db: &DatabaseConnection,
        question: &str,
        ai_solution: Option<&str>,
        question_type: &question::QuestionType,
        options: Option<&str>,
        level: &BloomLevel,
        topic: &str,
        content: &str,
    ) -> Result<question::Model, DbErr> {
        let quiz = question::ActiveModel {
            id: Set(Uuid::new_v4()),
            question: Set(question.to_string()),
            level: Set(*level),
            topic: Set(topic.to_string()),
            content: Set(content.to_string()),
            r#type: Set(*question_type),
            options: Set(options.map(ToString::to_string)),
            created_at: Set(chrono::Utc::now().naive_utc()),
            ai_solution: Set(ai_solution.map(ToString::to_string)),
        };
        quiz.insert(db).await
    }
}
