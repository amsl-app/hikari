use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::global::v01::frontend::FrontendConfigV01;

#[derive(Default, Serialize, Deserialize, Clone, ToSchema, Debug, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct VersionConfig {
    #[serde(default = "default_min_version")]
    /// # Minimum frontend version required
    pub min: String,
}

fn default_min_version() -> String {
    "0.0.0".to_string()
}

#[derive(Serialize, Deserialize, Clone, ToSchema, Debug, Default, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct FrontendConfig {
    pub frontend: VersionConfig,
}

impl From<FrontendConfigV01> for FrontendConfig {
    fn from(value: FrontendConfigV01) -> Self {
        Self {
            frontend: value.version,
        }
    }
}
