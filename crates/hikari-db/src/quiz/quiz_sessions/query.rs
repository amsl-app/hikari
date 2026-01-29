use hikari_entity::quiz::quiz_sessions::Column as QuizSessionsColumn;
use hikari_entity::quiz::quiz_sessions::Entity as QuizSessions;

use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter};

use uuid::Uuid;

pub struct Query;

impl Query {
    pub async fn get_quiz_sessions(db: &DatabaseConnection, quiz_id: &Uuid) -> Result<Vec<String>, DbErr> {
        let sessions = QuizSessions::find()
            .filter(QuizSessionsColumn::QuizId.eq(*quiz_id))
            .all(db)
            .await?;
        Ok(sessions.into_iter().map(|s| s.session_id).collect())
    }
}
