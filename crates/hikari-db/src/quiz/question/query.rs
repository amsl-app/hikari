use hikari_entity::quiz::question::{self, BloomLevel, Entity as Question, Model as QuestionModel};
use hikari_entity::quiz::quiz::{self};
use sea_orm::RelationTrait;
use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QuerySelect};
use std::error::Error;
use uuid::Uuid;
pub struct Query;

impl Query {
    pub async fn get_questions_by_quiz(db: &DatabaseConnection, quiz_id: &Uuid) -> Result<Vec<QuestionModel>, DbErr> {
        let query = Question::find().filter(question::Column::QuizId.eq(*quiz_id));
        query.all(db).await.inspect_err(|error| {
            tracing::error!(error = error as &dyn Error, "failed to load questions");
        })
    }

    pub async fn get_question_by_id(
        db: &DatabaseConnection,
        question_id: &Uuid,
    ) -> Result<Option<QuestionModel>, DbErr> {
        let query = Question::find().filter(question::Column::Id.eq(*question_id));

        let result = query.one(db).await.inspect_err(|error| {
            tracing::error!(error = error as &dyn Error, "failed to load question by id");
        })?;

        Ok(result)
    }

    pub async fn get_question_by_user_topic_level(
        db: &DatabaseConnection,
        user_id: &Uuid,
        topic: &str,
        level: &BloomLevel,
    ) -> Result<Vec<QuestionModel>, DbErr> {
        let query = Question::find()
            .join(sea_orm::JoinType::InnerJoin, question::Relation::Quiz.def())
            .filter(quiz::Column::UserId.eq(*user_id))
            .filter(question::Column::Topic.eq(topic.to_string()))
            .filter(question::Column::Level.eq(*level));

        let result = query.all(db).await.inspect_err(|error| {
            tracing::error!(
                error = error as &dyn Error,
                "failed to load question by user, topic and level"
            );
        })?;

        Ok(result)
    }

    pub async fn get_open_question(db: &DatabaseConnection, quiz_id: &Uuid) -> Result<Option<QuestionModel>, DbErr> {
        let query = Question::find()
            .filter(question::Column::QuizId.eq(*quiz_id))
            .filter(question::Column::Status.eq("open"));

        let result = query.one(db).await.inspect_err(|error| {
            tracing::error!(error = error as &dyn Error, "failed to load open question");
        })?;

        Ok(result)
    }
}
