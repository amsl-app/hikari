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
    type IdEnum: IdValue;
}

trait IdValue: FromStr + ToString {
    const ALLOWED_VALUES: &'static str;
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

fn allowed_fields_string(id_field: &str, settings: &[&str]) -> String {
    let mut fields = Vec::with_capacity(settings.len() + 1);
    fields.push(id_field.to_owned());
    fields.extend(settings.iter().map(|field| (*field).to_owned()));
    fields.join(", ")
}

fn missing_settings_string(settings: &[&str]) -> String {
    match settings {
        [] => String::new(),
        [single] => format!("'{single}'"),
        [first, second] => format!("'{first}' and/or '{second}'"),
        many => many
            .iter()
            .map(|setting| format!("'{setting}'"))
            .collect::<Vec<_>>()
            .join(", and/or "),
    }
}

macro_rules! join_literals {
    ($single:literal) => {
        $single
    };
    ($first:literal, $($rest:literal),+ $(,)?) => {
        concat!($first, ", ", join_literals!($($rest),+))
    };
}

macro_rules! define_id_enum {
    (
        $vis:vis enum $name:ident {
            $( $variant:ident = $value:literal ),+ $(,)?
        }
    ) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Display, EnumString)]
        $vis enum $name {
            $(
                #[strum(serialize = $value)]
                $variant,
            )+
        }

        impl IdValue for $name {
            const ALLOWED_VALUES: &'static str = join_literals!($( $value ),+);
        }
    };
}

macro_rules! impl_parse_from_parsed_values {
    (
        id_field: $id_field:ty,
        target: $target:ty,
        target_id: $target_id:ident,
        settings: { $( $map_key:literal => $field:ident ),+ $(,)? }
    ) => {
        impl TryFrom<ParsedValues<$id_field>> for $target {
            type Error = LlmArgParseError;

            fn try_from(mut parsed: ParsedValues<$id_field>) -> Result<Self, Self::Error> {
                const SETTINGS: &[&str] = &[ $( $map_key ),+ ];
                let allowed_fields = allowed_fields_string(<$id_field as IdField>::FIELD, SETTINGS);
                let missing_settings = missing_settings_string(SETTINGS);

                let id_str = parsed.id;
                let $target_id = id_str
                    .parse::<<$id_field as IdField>::IdEnum>()
                    .map_err(|_| LlmArgParseError::InvalidId {
                        kind: <$id_field as IdField>::FIELD,
                        value: id_str,
                        allowed_values: <<$id_field as IdField>::IdEnum as IdValue>::ALLOWED_VALUES,
                    })?;

                $(
                    let $field = parsed.values.remove($map_key);
                )+

                if let Some((unknown, _)) = parsed.values.into_iter().next() {
                    return Err(LlmArgParseError::UnknownField {
                        field: unknown,
                        allowed_fields,
                    });
                }

                if true $(&& $field.is_none())+ {
                    return Err(LlmArgParseError::MissingSetting {
                        kind: <$id_field as IdField>::FIELD,
                        id: $target_id.to_string(),
                        allowed_settings: missing_settings,
                    });
                }

                Ok(Self {
                    $target_id,
                    $( $field, )+
                })
            }
        }

        impl FromStr for $target {
            type Err = LlmArgParseError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                const SETTINGS: &[&str] = &[ $( $map_key ),+ ];

                let values = parse_key_value_map::<$id_field>(s).map_err(|error| match error {
                    KeyValueMapError::MissingSeparator(part) => LlmArgParseError::UnknownField {
                        field: part,
                        allowed_fields: allowed_fields_string(<$id_field as IdField>::FIELD, SETTINGS),
                    },
                    KeyValueMapError::EmptyValue(name) => LlmArgParseError::EmptyValue(name),
                    KeyValueMapError::MissingId => LlmArgParseError::MissingId(<$id_field as IdField>::FIELD),
                })?;

                Self::try_from(values)
            }
        }
    };
}

struct ServiceIdField;

impl IdField for ServiceIdField {
    const FIELD: &'static str = "service";
    type IdEnum = LlmServiceType;
}

#[derive(Debug, Clone)]
pub struct LlmServiceArg {
    pub service: LlmServiceType,
    pub key: Option<String>,
    pub default_model: Option<String>,
}

define_id_enum!(
    pub enum LlmServiceType {
        Openai = "openai",
        Gwdg = "gwdg",
        Kit = "kit",
    }
);

#[derive(Debug, Error)]
pub enum LlmArgParseError {
    #[error("Missing required '{0}' field")]
    MissingId(&'static str),
    #[error("At least one setting is required for {kind} '{id}'. Set {allowed_settings}")]
    MissingSetting {
        kind: &'static str,
        id: String,
        allowed_settings: String,
    },
    #[error("Unknown field '{field}'. Allowed fields: {allowed_fields}")]
    UnknownField { field: String, allowed_fields: String },
    #[error("Field '{0}' must not be empty")]
    EmptyValue(String),
    #[error("Unknown {kind} '{value}'. Allowed {kind}s: {allowed_values}")]
    InvalidId {
        kind: &'static str,
        value: String,
        allowed_values: &'static str,
    },
}

impl_parse_from_parsed_values!(
    id_field: ServiceIdField,
    target: LlmServiceArg,
    target_id: service,
    settings: {
        "key" => key,
        "default-model" => default_model
    }
);

struct FeatureIdField;

impl IdField for FeatureIdField {
    const FIELD: &'static str = "feature";
    type IdEnum = LlmFeatureType;
}

#[derive(Debug, Clone)]
pub struct LlmFeatureArg {
    pub feature: LlmFeatureType,
    pub service: Option<String>,
    pub model: Option<String>,
}

define_id_enum!(
    pub enum LlmFeatureType {
        Journaling = "journaling",
        Embedding = "embedding",
        Quiz = "quiz",
    }
);

impl_parse_from_parsed_values!(
    id_field: FeatureIdField,
    target: LlmFeatureArg,
    target_id: feature,
    settings: {
        "service" => service,
        "model" => model
    }
);

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
        let cli = TestCli::try_parse_from(["test-bin", "--llm-config=service=kit,key=abc,default-model=model-a"])
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
        assert_eq!(
            cli.llm_services.llm_feature_config[0].feature,
            LlmFeatureType::Embedding
        );
        assert_eq!(cli.llm_services.llm_feature_config[1].feature, LlmFeatureType::Quiz);
        assert_eq!(
            cli.llm_services.llm_feature_config[0].service.as_deref(),
            Some("openai")
        );
        assert_eq!(
            cli.llm_services.llm_feature_config[1].model.as_deref(),
            Some("gpt-4.1-mini")
        );
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
