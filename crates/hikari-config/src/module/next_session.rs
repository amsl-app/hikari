use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::module::v01::next_session::NextSessionV01;

#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct NextSessionFull {
    pub id: String,
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Deserialize, JsonSchema, Serialize, Clone)]
pub enum NextSession {
    Simple(String),
    Full(NextSessionFull),
}

impl NextSession {
    pub(crate) fn from_v01(session: NextSessionV01) -> Self {
        match session {
            NextSessionV01::Simple(id) => NextSession::Simple(id),
            NextSessionV01::Full(full) => NextSession::Full(full),
        }
    }
}
