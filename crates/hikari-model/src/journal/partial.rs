use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NewJournalContent {
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NewJournalEntryWithData {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub content: Vec<NewJournalContent>,
    #[serde(default)]
    pub focus: Vec<Uuid>,
    #[serde(default)]
    pub mood: Option<f32>,
    #[serde(default)]
    pub prompts: Vec<String>,
}
