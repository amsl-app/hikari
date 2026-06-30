use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(crate) struct NextSessionFullV01 {
    pub module_id: Option<String>,
    pub session_id: String,
    pub force: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(untagged)]
pub(crate) enum NextSessionV01 {
    Simple(String),
    Full(NextSessionFullV01),
}
