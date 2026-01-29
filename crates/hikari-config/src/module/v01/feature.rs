use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    generic::Theme,
    module::v01::{llm_agent::LlmAgentV01, unlock::UnlockV01},
};

#[derive(Serialize, Deserialize, Debug, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(crate) struct FeatureV01 {
    #[serde(default)]
    /// # Whether the feature is enabled
    pub(crate) enabled: bool,

    #[serde(default, flatten)]
    /// # Optional override of the default agent for the feature
    /// Similar to the `llm_agent` field in sessions
    pub(crate) llm_agent: Option<LlmAgentV01>,

    #[serde(default)]
    /// # Unlock conditions for the feature
    /// Similar to unlock conditions for sessions
    pub(crate) unlock: Option<UnlockV01>,

    #[serde(default)]
    /// # Theme of the feature
    /// Similar to a theme of a session or module
    pub(crate) theme: Option<Theme>,
}
