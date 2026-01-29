use hikari_config::module::content::QuestionBloomLevel;
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
pub enum QuestionType {
    Text,
    MultipleChoice,
}

#[derive(Deserialize, Serialize, ToSchema, Clone)]
#[serde(rename_all = "snake_case")]
pub struct QuestionOption {
    pub option: String,
    pub correct: Option<bool>,
}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct Question {
    pub id: Uuid,
    pub quiz_id: Uuid,
    pub session_id: String,
    pub topic: String,
    pub content: String,
    pub question: String,
    pub r#type: QuestionType,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub options: Vec<QuestionOption>,
    pub level: QuestionBloomLevel,
    pub created_at: chrono::NaiveDateTime,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub answered_at: Option<chrono::NaiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub answer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evaluation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grade: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai_solution: Option<String>,
    pub status: QuestionStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feedback: Option<QuestionFeedback>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feedback_explanation: Option<String>,
}

impl Question {
    pub fn sanitize_for_client(&mut self) {
        let is_graded = self.evaluation.is_some() || self.grade.is_some();

        if !is_graded {
            self.ai_solution = None;
            for option in &mut self.options {
                option.correct = None;
            }
        }
    }
}
