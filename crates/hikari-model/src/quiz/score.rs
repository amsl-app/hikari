use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct Score {
    pub user_id: Uuid,
    pub module_id: String,
    pub session_id: String,
    pub topic: String,
    pub score: f64,
}
