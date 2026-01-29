use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct ItemValue {
    pub id: String,
    pub title: String,
    pub value: f64,
}
