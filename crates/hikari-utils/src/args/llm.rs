use clap::Args;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::str::FromStr;
use strum::{Display, EnumString};
use thiserror::Error;
use url::Url;

#[derive(Debug, Clone, Args)]
pub struct LlmServices {
    #[arg(long, required = false)]
    pub llm_config: Vec<LlmServiceArg>,
    #[arg(long, required = false)]
    pub llm_feature_config: Vec<LlmFeatureArg>,
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

#[derive(Debug)]
enum KeyValueMapError {
    MissingSeparator(String),
    EmptyValue(String),
    MissingId,
}

trait IdField {
    const FIELD: &'static str;
}

struct ParsedValues<I: IdField> {
    id: String,
    values: HashMap<String, String>,
    _marker: PhantomData<I>,
}

fn parse_key_value_map<I: IdField>(input: &str) -> Result<ParsedValues<I>, KeyValueMapError> {
    let mut values = HashMap::new();

    for part in input.split(',').map(str::trim).filter(|part| !part.is_empty()) {
        let (name, value) = part
            .split_once('=')
            .map(|(name, value)| (name.trim(), value.trim()))
            .ok_or_else(|| KeyValueMapError::MissingSeparator(part.to_string()))?;

        if value.is_empty() {
            return Err(KeyValueMapError::EmptyValue(name.to_string()));
        }

        values.insert(name.to_owned(), value.to_owned());
    }

    let id = values.remove(I::FIELD).ok_or(KeyValueMapError::MissingId)?;

    Ok(ParsedValues {
        id,
        values,
        _marker: PhantomData,
    })
}

struct ServiceIdField;

impl IdField for ServiceIdField {
    const FIELD: &'static str = "service";
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

impl TryFrom<ParsedValues<ServiceIdField>> for LlmServiceArg {
    type Error = LlmServiceArgError;

    fn try_from(mut parsed: ParsedValues<ServiceIdField>) -> Result<Self, Self::Error> {
        let service_str = parsed.id;
        let service = service_str
            .parse::<LlmServiceType>()
            .map_err(|_| LlmServiceArgError::InvalidService(service_str))?;

        let key = parsed.values.remove("key");
        let default_model = parsed.values.remove("default-model");

        if let Some((unknown, _)) = parsed.values.into_iter().next() {
            return Err(LlmServiceArgError::UnknownField(unknown));
        }
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

impl FromStr for LlmServiceArg {
    type Err = LlmServiceArgError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let values = parse_key_value_map::<ServiceIdField>(s).map_err(|error| match error {
            KeyValueMapError::MissingSeparator(part) => LlmServiceArgError::UnknownField(part),
            KeyValueMapError::EmptyValue(name) => LlmServiceArgError::EmptyValue(name),
            KeyValueMapError::MissingId => LlmServiceArgError::MissingService,
        })?;
        Self::try_from(values)
    }
}

struct FeatureIdField;

impl IdField for FeatureIdField {
    const FIELD: &'static str = "feature";
}

#[derive(Debug, Clone)]
pub struct LlmFeatureArg {
    pub feature: LlmFeatureType,
    pub service: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, EnumString)]
#[strum(serialize_all = "lowercase")]
pub enum LlmFeatureType {
    Journaling,
    Embedding,
    Quiz,
}

#[derive(Debug, Error)]
pub enum LlmFeatureArgError {
    #[error("Missing required 'feature' field")]
    MissingFeature,
    #[error("At least one setting is required for feature '{0}'. Set 'service' and/or 'model'")]
    MissingSetting(LlmFeatureType),
    #[error("Unknown field '{0}'. Allowed fields: feature, service, model")]
    UnknownField(String),
    #[error("Field '{0}' must not be empty")]
    EmptyValue(String),
    #[error("Unknown feature '{0}'. Allowed features: journaling, embedding, quiz")]
    InvalidFeature(String),
}

impl TryFrom<ParsedValues<FeatureIdField>> for LlmFeatureArg {
    type Error = LlmFeatureArgError;

    fn try_from(mut parsed: ParsedValues<FeatureIdField>) -> Result<Self, Self::Error> {
        let feature_str = parsed.id;
        let feature = feature_str
            .parse::<LlmFeatureType>()
            .map_err(|_| LlmFeatureArgError::InvalidFeature(feature_str))?;

        let service = parsed.values.remove("service");
        let model = parsed.values.remove("model");

        if let Some((unknown, _)) = parsed.values.into_iter().next() {
            return Err(LlmFeatureArgError::UnknownField(unknown));
        }
        if service.is_none() && model.is_none() {
            return Err(LlmFeatureArgError::MissingSetting(feature));
        }

        Ok(Self {
            feature,
            service,
            model,
        })
    }
}

impl FromStr for LlmFeatureArg {
    type Err = LlmFeatureArgError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let values = parse_key_value_map::<FeatureIdField>(s).map_err(|error| match error {
            KeyValueMapError::MissingSeparator(part) => LlmFeatureArgError::UnknownField(part),
            KeyValueMapError::EmptyValue(name) => LlmFeatureArgError::EmptyValue(name),
            KeyValueMapError::MissingId => LlmFeatureArgError::MissingFeature,
        })?;
        Self::try_from(values)
    }
}

#[cfg(test)]
mod tests {
    use super::{LlmFeatureType, LlmServiceType, LlmServices};
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
            "--other",
            "bar",
            "--llm-config=service=gwdg,default-model=llama",
        ])
        .expect("repeated llm-config should parse");

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

    #[test]
    fn test_parses_single_llm_feature_config() {
        let cli = TestCli::try_parse_from([
            "test-bin",
            "--llm-feature-config=feature=journaling,service=kit,model=model-a",
        ])
        .expect("llm-feature-config should parse");

        let cfg = &cli.llm_services.llm_feature_config[0];
        assert_eq!(cfg.feature, LlmFeatureType::Journaling);
        assert_eq!(cfg.service.as_deref(), Some("kit"));
        assert_eq!(cfg.model.as_deref(), Some("model-a"));
    }

    #[test]
    fn test_parses_repeated_llm_feature_config_values() {
        let cli = TestCli::try_parse_from([
            "test-bin",
            "--llm-feature-config=feature=embedding,service=openai",
            "--llm-feature-config=feature=quiz,model=gpt-4.1-mini",
        ])
        .expect("repeated llm-feature-config should parse");

        assert_eq!(cli.llm_services.llm_feature_config.len(), 2);
        assert_eq!(cli.llm_services.llm_feature_config[0].feature, LlmFeatureType::Embedding);
        assert_eq!(cli.llm_services.llm_feature_config[1].feature, LlmFeatureType::Quiz);
        assert_eq!(cli.llm_services.llm_feature_config[0].service.as_deref(), Some("openai"));
        assert_eq!(cli.llm_services.llm_feature_config[1].model.as_deref(), Some("gpt-4.1-mini"));
    }

    #[test]
    fn test_rejects_missing_feature() {
        let result = TestCli::try_parse_from(["test-bin", "--llm-feature-config=service=openai"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_rejects_invalid_feature() {
        let result = TestCli::try_parse_from(["test-bin", "--llm-feature-config=feature=foo,service=openai"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_rejects_missing_feature_setting() {
        let result = TestCli::try_parse_from(["test-bin", "--llm-feature-config=feature=quiz"]);
        assert!(result.is_err());
    }
}
