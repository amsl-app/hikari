use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Deserialize, ToSchema)]
pub struct QuizSession {
    pub quiz_id: Uuid,
    pub session_id: String,
}
