use std::collections::HashMap;

use hikari_utils::id_map::ItemId;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::{
    generic::{Metadata, Theme},
    module::v01::{llm_agent::LlmAgentV01, unlock::UnlockV01},
};

pub fn validate_session_id<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let id: String = serde::Deserialize::deserialize(deserializer)?;
    if id == "self-learning" {
        return Err(serde::de::Error::custom(format!(
            "The session ID '{id}' is reserved for the self-learning session. Please choose a different ID."
        )));
    }
    Ok(id)
}

fn default_quizzable_session() -> bool {
    true
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(crate) struct SessionV01 {
    #[serde(deserialize_with = "validate_session_id")]
    /// # Unique identifier of the session
    pub(crate) id: String,
    /// # Title of the session
    pub(crate) title: String,
    /// # Subtitle of the session
    pub(crate) subtitle: Option<String>,
    /// # Description of the session
    pub(crate) description: Option<String>,
    /// # Icon associated with the session
    pub(crate) icon: Option<String>,
    /// # Banner image associated with the session
    pub(crate) banner: Option<String>,
    /// # Depricated bot associated with the session
    pub(crate) bot: Option<String>,
    #[allow(clippy::struct_field_names)]
    #[serde(rename = "next-session")]
    /// # Next session for quick navigation after completing this session
    pub(crate) next_session: Option<String>,
    /// # Theme of the session  
    pub(crate) theme: Option<Theme>,
    /// # Estimated time to complete the session in minutes
    pub(crate) time: Option<i32>,
    #[serde(default)]
    /// # Whether the session is hidden from the frontend
    pub(crate) hidden: bool,
    #[serde(default = "default_quizzable_session")]
    /// # Whether the session is included in quizzes
    /// Only relevant if the module is quizzable
    pub(crate) quizzable: bool,
    /// # Unlock conditions for the session
    pub(crate) unlock: Option<UnlockV01>,
    pub(crate) metadata: Option<Metadata>,
    #[serde(default, flatten)]
    /// # LLM agent configuration for the session
    /// LLM agents define the structure of the chatbots
    pub(crate) llm_agent: Option<LlmAgentV01>,
    #[serde(default)]
    /// # Contents covered in the session
    /// References to content IDs defined in the module
    pub(crate) contents: Vec<String>,
    #[schemars(with = "Option<HashMap<String, serde_json::Value>>")]
    pub(crate) custom: Option<HashMap<String, serde_yml::Value>>,
}

impl ItemId for SessionV01 {
    type IdType = String;

    fn id(&self) -> Self::IdType {
        self.id.clone()
    }
}
