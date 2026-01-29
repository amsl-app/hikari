use std::str::FromStr;

use async_openai::config::OpenAIConfig;
use hikari_config::module::llm_agent::LlmService;
use hikari_utils::args::llm::LlmServices as LlmServiceArgs;
#[derive(Debug, Clone)]
pub struct LlmServiceConfig {
    pub key: Option<String>,
    pub default_model: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LlmFeatureConfig {
    pub service: Option<LlmService>,
    pub model: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LlmConfig {
    openai: LlmServiceConfig,
    gwdg: LlmServiceConfig,
    custom: LlmServiceConfig,
    pub embedding_config: LlmFeatureConfig,
    pub journaling_config: LlmFeatureConfig,
    pub quiz_config: LlmFeatureConfig,
}

impl From<LlmServiceArgs> for LlmConfig {
    fn from(config: LlmServiceArgs) -> LlmConfig {
        let embedding_service = config
            .embedding_service
            .as_deref()
            .map(|s| LlmService::from_str(s).expect("Invalid embedding_service string"));

        let journaling_service = config
            .journaling_service
            .as_deref()
            .map(|s| LlmService::from_str(s).expect("Invalid journaling_service string"));

        let quiz_service = config
            .quiz_service
            .as_deref()
            .map(|s| LlmService::from_str(s).expect("Invalid quiz_service string"));

        Self {
            openai: LlmServiceConfig {
                key: config.openai_key,
                default_model: config.openai_default_model,
            },
            gwdg: LlmServiceConfig {
                key: config.gwdg_key,
                default_model: config.gwdg_default_model,
            },
            custom: LlmServiceConfig {
                key: config.win_key,
                default_model: config.win_default_model,
            },
            embedding_config: LlmFeatureConfig {
                service: embedding_service,
                model: config.embedding_model,
            },
            journaling_config: LlmFeatureConfig {
                service: journaling_service,
                model: config.journaling_model,
            },
            quiz_config: LlmFeatureConfig {
                service: quiz_service,
                model: config.quiz_model,
            },
        }
    }
}

impl LlmConfig {
    #[must_use]
    pub fn new(
        openai: LlmServiceConfig,
        gwdg: LlmServiceConfig,
        custom: LlmServiceConfig,
        embeddings: LlmFeatureConfig,
        journaling: LlmFeatureConfig,
        quiz: LlmFeatureConfig,
    ) -> Self {
        Self {
            openai,
            gwdg,
            custom,
            embedding_config: embeddings,
            journaling_config: journaling,
            quiz_config: quiz,
        }
    }

    #[must_use]
    pub fn get_default_model(&self, service: Option<&LlmService>) -> &str {
        let default = LlmService::default();
        let service = service.unwrap_or(&default);
        match service {
            LlmService::OpenAI => self.openai.default_model.as_deref().unwrap_or("gpt-4.1-mini"),
            LlmService::Gwdg => self.gwdg.default_model.as_deref().unwrap_or("llama-3.3-70b-instruct"),
            LlmService::Custom(_) => self.custom.default_model.as_deref().unwrap_or("llama-3.3-8b-instruct"),
        }
    }

    #[must_use]
    pub fn get_key(&self, service: &LlmService) -> Option<&str> {
        match service {
            LlmService::OpenAI => self.openai.key.as_deref(),
            LlmService::Gwdg => self.gwdg.key.as_deref(),
            LlmService::Custom(_) => self.custom.key.as_deref(),
        }
    }

    #[must_use]
    pub fn get_openai_config(&self, service: Option<&LlmService>) -> OpenAIConfig {
        let default = LlmService::default();
        let service = service.unwrap_or(&default);
        let mut openai_config = OpenAIConfig::default().with_api_base(service.get_base());

        if let Some(api_key) = self.get_key(service) {
            openai_config = openai_config.with_api_key(api_key);
        }
        openai_config
    }

    #[must_use]
    pub fn get_embedding_model(&self) -> &str {
        if let Some(model) = &self.embedding_config.model {
            model.as_str()
        } else {
            tracing::debug!("Using default model for embedding feature");
            self.get_default_model(self.embedding_config.service.as_ref())
        }
    }

    #[must_use]
    pub fn get_embedding_openai_config(&self) -> OpenAIConfig {
        self.get_openai_config(self.embedding_config.service.as_ref())
    }

    #[must_use]
    pub fn get_journaling_model(&self) -> &str {
        if let Some(model) = &self.journaling_config.model {
            model.as_str()
        } else {
            tracing::debug!("Using default model for journaling feature");
            self.get_default_model(self.journaling_config.service.as_ref())
        }
    }

    #[must_use]
    pub fn get_journaling_openai_config(&self) -> OpenAIConfig {
        self.get_openai_config(self.journaling_config.service.as_ref())
    }

    #[must_use]
    pub fn get_quiz_model(&self) -> &str {
        if let Some(model) = &self.quiz_config.model {
            model.as_str()
        } else {
            tracing::debug!("Using default model for quiz feature");
            self.get_default_model(self.quiz_config.service.as_ref())
        }
    }

    #[must_use]
    pub fn get_quiz_openai_config(&self) -> OpenAIConfig {
        self.get_openai_config(self.quiz_config.service.as_ref())
    }
}
