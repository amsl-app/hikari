use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;


#[derive(Deserialize, Serialize, ToSchema, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum QuestionFeedback {
    Good,
    Bad,
}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum QuestionStatus {
    Open,
    Finished,
    Skipped,
}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct QuizQuestionAttempt {
    pub question_id: Uuid,
    pub quiz_id: Uuid,
    pub attempt: i32,
    pub session_id: String,
    pub asked_at:Option<chrono::NaiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub answered_at: Option<chrono::NaiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub answer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evaluation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grade: Option<i32>,
    pub status: QuestionStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feedback: Option<QuestionFeedback>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feedback_explanation: Option<String>,
}

impl QuizQuestionAttempt {
    
}