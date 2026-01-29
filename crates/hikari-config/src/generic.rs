use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema, Debug, Clone, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Theme {
    #[schema(example = "theme-id")]
    pub id: String,
}

#[derive(Serialize, Deserialize, ToSchema, Debug, Clone, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Metadata {
    #[serde(default)]
    pub annotations: HashMap<String, String>,
}
