use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct QuestionRecommendation {
    pub id: Uuid,
    pub quiz_id: Uuid,
    pub question_id: Uuid,
    pub recommended_at: chrono::NaiveDateTime,
    pub used: bool,
    pub used_at: Option<chrono::NaiveDateTime>,
}

impl QuestionRecommendation {
    
}