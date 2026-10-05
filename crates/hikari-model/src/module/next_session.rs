use hikari_config::module::next_session::Next as NextConfig;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Next {
    pub module_id: String,
    pub session_id: String,
    pub force: bool,
}

impl Next {
    pub fn from_config(next: &NextConfig) -> Self {
        Next {
            module_id: next.module_id.clone(),
            session_id: next.session_id.clone(),
            force: next.force,
        }
    }
}
