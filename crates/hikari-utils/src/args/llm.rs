use clap::Args;
use std::str::FromStr;
use strum::{Display, EnumString};
use thiserror::Error;
use url::Url;

#[derive(Debug, Clone, Args)]
pub struct LlmServices {
    #[arg(long, required = false)]
    pub llm_config: Vec<LlmServiceArg>,
    #[arg(long, required = false)]
    pub journaling_model: Option<String>,
    #[arg(long, required = false)]
    pub journaling_service: Option<String>,
    #[arg(long, required = false)]
    pub embedding_model: Option<String>,
    #[arg(long, required = false)]
    pub embedding_service: Option<String>,
    #[arg(long, required = false)]
    pub quiz_model: Option<String>,
    #[arg(long, required = false)]
    pub quiz_service: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LlmServiceArg {
    pub service: LlmServiceType,
    pub key: Option<String>,
    pub default_model: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, EnumString)]
#[strum(serialize_all = "lowercase")]
pub enum LlmServiceType {
    Openai,
    Gwdg,
    Kit,
}

#[derive(Debug, Error)]
pub enum LlmServiceArgError {
    #[error("Missing required 'service' field")]
    MissingService,
    #[error("At least one setting is required for service '{0}'. Set 'key' and/or 'default-model'")]
    MissingSetting(LlmServiceType),
    #[error("Unknown field '{0}'. Allowed fields: service, key, default-model")]
    UnknownField(String),
    #[error("Field '{0}' must not be empty")]
    EmptyValue(String),
    #[error("Unknown service '{0}'. Allowed services: openai, gwdg, kit")]
    InvalidService(String),
}

impl FromStr for LlmServiceArg {
    type Err = LlmServiceArgError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut service = None;
        let mut key = None;
        let mut default_model = None;

        for part in s.split(',').map(str::trim).filter(|part| !part.is_empty()) {
            let (name, value) = part
                .split_once('=')
                .map(|(name, value)| (name.trim(), value.trim()))
                .ok_or_else(|| LlmServiceArgError::UnknownField(part.to_string()))?;

            if value.is_empty() {
                return Err(LlmServiceArgError::EmptyValue(name.to_string()));
            }

            match name {
                "service" => {
                    service = Some(
                        value
                            .parse::<LlmServiceType>()
                            .map_err(|_| LlmServiceArgError::InvalidService(value.to_owned()))?,
                    )
                }
                "key" => key = Some(value.to_owned()),
                "default-model" => default_model = Some(value.to_owned()),
                _ => return Err(LlmServiceArgError::UnknownField(name.to_string())),
            }
        }

        let Some(service) = service else {
            return Err(LlmServiceArgError::MissingService);
        };
        if key.is_none() && default_model.is_none() {
            return Err(LlmServiceArgError::MissingSetting(service));
        }

        Ok(Self {
            service,
            key,
            default_model,
        })
    }
}

#[derive(Debug, Clone, Args)]
#[allow(clippy::struct_field_names)]
pub struct LlmConfig {
    #[arg(long, required = false)]
    pub llm_structures: Option<Url>,
    #[arg(long, required = false)]
    pub llm_collections: Url,
    #[arg(long, required = false, help = "The url were the constants are stored")]
    pub constants: Option<Url>,
}

#[cfg(test)]
mod tests {
    use super::{LlmServiceType, LlmServices};
    use clap::Parser;

    #[derive(Debug, Parser)]
    struct TestCli {
        #[command(flatten)]
        llm_services: LlmServices,
        #[arg(long, required = false)]
        other: Option<String>,
    }

    #[test]
    fn test_parses_single_llm_config() {
        let cli = TestCli::try_parse_from([
            "test-bin",
            "--llm-config=service=kit,key=abc,default-model=model-a",
        ])
            .expect("llm-config should parse");

        let cfg = &cli.llm_services.llm_config[0];
        assert_eq!(cfg.service, LlmServiceType::Kit);
        assert_eq!(cfg.key.as_deref(), Some("abc"));
        assert_eq!(cfg.default_model.as_deref(), Some("model-a"));
    }

    #[test]
    fn test_parses_repeated_llm_config_values() {
        let cli = TestCli::try_parse_from([
            "test-bin",
            "--llm-config=service=openai,key=openai-key",
            "--other", "bar",
            "--llm-config=service=gwdg,default-model=llama",
        ]).expect("repeated llm-config should parse");

        assert_eq!(cli.llm_services.llm_config.len(), 2);
        assert_eq!(cli.llm_services.llm_config[0].service, LlmServiceType::Openai);
        assert_eq!(cli.llm_services.llm_config[1].service, LlmServiceType::Gwdg);
        assert_eq!(cli.llm_services.llm_config[0].key.as_deref(), Some("openai-key"));
        assert_eq!(cli.llm_services.llm_config[1].default_model.as_deref(), Some("llama"));
    }

    #[test]
    fn test_rejects_missing_service() {
        let result = TestCli::try_parse_from(["test-bin", "--llm-config=key=abc"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_rejects_invalid_service() {
        let result = TestCli::try_parse_from(["test-bin", "--llm-config=service=foo,key=abc"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_rejects_missing_setting() {
        let result = TestCli::try_parse_from(["test-bin", "--llm-config=service=kit"]);
        assert!(result.is_err());
    }
}
