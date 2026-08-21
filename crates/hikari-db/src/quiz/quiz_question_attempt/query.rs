use sea_orm::{DatabaseConnection, DbErr, EntityTrait, QuerySelect};
use hikari_entity::quiz::quiz_question_attempt::{Entity as Attempt, Model as AttemptModel};
use std::error::Error;
use uuid::Uuid;
pub struct Query;

impl Query {
    pub async fn get_attempt(
        db: &DatabaseConnection,
        quiz_id: &Uuid,
        question_id: &Uuid,
        attempt: &i32,
    ) -> Result<Option<AttemptModel>, DbErr> {
        Attempt::find_by_id((
            *quiz_id,
            *question_id,
            *attempt,
        ))
            .one(db)
            .await
            .inspect_err(|error| {
                tracing::error!(
            error = error as &dyn Error,
            "failed to load question attempt"
        );
            })
    }

    pub async fn get_session_id_by_attempt(
        db: &DatabaseConnection,
        quiz_id: &Uuid,
        question_id: &Uuid,
        attempt: &i32,
    ) -> Result<Option<String>, DbErr> {
        let result = Attempt::find_by_id((
            *quiz_id,
            *question_id,
            *attempt,
        ))
            .select_only()
            .column(<hikari_entity::quiz::quiz_question_attempt::Entity as sea_orm::EntityTrait>::Column::SessionId)
            .into_tuple::<String>()
            .one(db)
            .await
            .inspect_err(|error| {
                tracing::error!(
            error = error as &dyn Error,
            "failed to load session id by attempt"
        );
            })?;

        Ok(result)
    }
}
