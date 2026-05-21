use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserContextLog {
    pub user_id: Uuid,
    pub created_at: NaiveDateTime,
    pub r#type: String,
    pub data: Value,
}
