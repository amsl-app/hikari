use schemars::JsonSchema;
use serde::Deserialize;

use crate::global::frontend::VersionConfig;

#[derive(Deserialize, Clone, Debug, Default, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct FrontendConfigV01 {
    pub version: VersionConfig,
}
