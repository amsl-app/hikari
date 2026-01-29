use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Tag {
    pub id: Uuid,
    pub name: String,
    pub user_id: Option<Uuid>,
    pub icon: String,
    pub hidden: bool,
}
