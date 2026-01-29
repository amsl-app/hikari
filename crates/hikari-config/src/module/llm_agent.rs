use std::borrow::Cow;
use std::str::FromStr;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use url::Url;
use utoipa::ToSchema;

use crate::module::error::LlmServiceError;

#[derive(Serialize, Deserialize, Default, Debug, Clone, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct LlmAgent {
    /// # Identifier of the LLM agent to be used
    /// References an LLM agent
    /// LLM agents are defined in sepearte *.agent.yaml files
    pub llm_agent: String,
    #[serde(default)]
    pub provider: LlmService,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "lowercase")]
pub enum LlmService {
    #[default]
    OpenAI,
    Gwdg,
    Custom(Url),
}

impl FromStr for LlmService {
    type Err = LlmServiceError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "openai" => Ok(LlmService::OpenAI),
            "gwdg" => Ok(LlmService::Gwdg),
            _ => Err(LlmServiceError::UnknownService(s.to_string())),
        }
    }
}

impl LlmService {
    #[must_use]
    pub fn get_base(&self) -> Cow<'_, str> {
        match self {
            LlmService::OpenAI => "https://api.openai.com/v1".into(),
            LlmService::Gwdg => "https://chat-ai.academiccloud.de/v1".into(),
            LlmService::Custom(url) => Cow::from(url.as_str()),
        }
    }
}
