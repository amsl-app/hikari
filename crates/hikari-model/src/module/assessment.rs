use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use utoipa::ToSchema;
use uuid::Uuid;

pub mod instance;
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct ModuleAssessmentFull<'a> {
    pub pre: Cow<'a, str>,
    pub post: Cow<'a, str>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_pre: Option<Uuid>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_post: Option<Uuid>,
}
