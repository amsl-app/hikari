use sea_orm::{ActiveModelTrait, ConnectionTrait, TransactionTrait};
use uuid::Uuid;

pub struct Mutation;

impl Mutation {
    pub async fn add_quiz_session<C: ConnectionTrait + TransactionTrait>(
        db: &C,
        quiz_id: &Uuid,
        session_id: &str,
    ) -> Result<(), sea_orm::DbErr> {
        let quiz_session = hikari_entity::quiz::quiz_sessions::ActiveModel {
            quiz_id: sea_orm::Set(*quiz_id),
            session_id: sea_orm::Set(session_id.to_string()),
        };

        quiz_session.insert(db).await?;

        Ok(())
    }
}
