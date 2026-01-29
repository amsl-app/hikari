use hikari_entity::quiz::quiz;
use hikari_entity::quiz::quiz::{Entity as Quiz, Model as QuizModel};
use hikari_entity::quiz::quiz_sessions::Entity as QuizSessions;
use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter};
use std::error::Error;
use uuid::Uuid;
pub struct Query;

impl Query {
    pub async fn get_quizzes(db: &DatabaseConnection, user_id: &Uuid) -> Result<Vec<QuizModel>, DbErr> {
        let query = Quiz::find().filter(quiz::Column::UserId.eq(*user_id));
        query.all(db).await.inspect_err(|error| {
            tracing::error!(error = error as &dyn Error, "failed to load quizzes");
        })
    }

    pub async fn get_quizzes_by_module(
        db: &DatabaseConnection,
        user_id: &Uuid,
        module_id: &str,
    ) -> Result<Vec<QuizModel>, DbErr> {
        let query = Quiz::find()
            .filter(quiz::Column::ModuleId.eq(module_id))
            .filter(quiz::Column::UserId.eq(*user_id));
        query.all(db).await.inspect_err(|error| {
            tracing::error!(error = error as &dyn Error, "failed to load quizzes by module");
        })
    }

    pub async fn get_quiz_by_id(
        db: &DatabaseConnection,
        user_id: &Uuid,
        quiz_id: &Uuid,
    ) -> Result<Option<QuizModel>, DbErr> {
        let query = Quiz::find()
            .filter(quiz::Column::Id.eq(*quiz_id))
            .filter(quiz::Column::UserId.eq(*user_id));
        query.one(db).await.inspect_err(|error| {
            tracing::error!(error = error as &dyn Error, "failed to load quiz by id");
        })
    }

    pub async fn get_quiz_sessions(db: &DatabaseConnection, quiz_id: &Uuid) -> Result<Vec<String>, DbErr> {
        let sessions = QuizSessions::find()
            .filter(<QuizSessions as sea_orm::EntityTrait>::Column::QuizId.eq(*quiz_id))
            .all(db)
            .await?;

        Ok(sessions.into_iter().map(|s| s.session_id).collect())
    }
}
