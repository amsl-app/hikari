use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Default, Serialize, Deserialize, Clone, ToSchema, Debug, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct UserConfig {
    /// # Allowed keys for custom user config
    /// `UserConfigs` can store json values and objects under these keys
    /// The values can be used in prompts for agents
    pub allowed_keys: Vec<String>,
}
