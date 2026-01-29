pub mod content;
pub mod partial;

use chrono::FixedOffset;
use serde::{Deserialize, Serialize};

use crate::tag::Tag;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct JournalEntry {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: Option<String>,
    pub mood: Option<f32>,
    pub created_at: chrono::DateTime<FixedOffset>,
    pub updated_at: chrono::DateTime<FixedOffset>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MetaContent {
    pub id: Uuid,
    pub journal_entry_id: Uuid,
    pub created_at: chrono::DateTime<FixedOffset>,
    pub updated_at: chrono::DateTime<FixedOffset>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, ToSchema)]
pub struct MetaJournalEntryWithMetaContent {
    pub id: Uuid,
    pub user_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub title: Option<String>,
    pub created_at: chrono::DateTime<FixedOffset>,
    pub updated_at: chrono::DateTime<FixedOffset>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub content: Vec<MetaContent>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub focus: Vec<Tag>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub mood: Option<f32>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub prompts: Vec<String>,
}
