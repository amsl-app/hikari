use chrono::FixedOffset;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct JournalContent {
    pub id: Uuid,
    pub journal_entry_id: Uuid,
    pub title: Option<String>,
    pub content: String,
    pub created_at: chrono::DateTime<FixedOffset>,
    pub updated_at: chrono::DateTime<FixedOffset>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct JournalContentId {
    pub id: Uuid,
}
