use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::module::v01::next_session::NextSessionV01;

#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Next {
    pub module_id: String,
    pub session_id: String,
    #[serde(default)]
    pub force: bool,
}

impl Next {
    pub(crate) fn from_v01(session: NextSessionV01, module_id: &str) -> Self {
        match session {
            NextSessionV01::Simple(id) => Next {
                module_id: module_id.to_owned(),
                session_id: id,
                force: false,
            },
            NextSessionV01::Full(full) => Next {
                module_id: full.module_id.unwrap_or_else(|| module_id.to_owned()),
                session_id: full.session_id,
                force: full.force,
            },
        }
    }
}
