use schemars::JsonSchema;
use serde::Deserialize;

use crate::module::next_session::NextSessionFull;


pub(crate) type NextSessionFullV01 = NextSessionFull;

#[derive(Deserialize, JsonSchema)]
#[serde(untagged)]
pub(crate) enum NextSessionV01 {
    Simple(String),
    Full(NextSessionFullV01),
}
