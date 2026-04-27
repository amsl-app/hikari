use clap::Args;
use std::collections::HashMap;
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

trait IdValue: FromStr + ToString {
    const ALLOWED_VALUES: &'static str;
}

struct ParsedValues {
    id: String,
    values: HashMap<String, String>,
}

fn parse_key_value_map(input: &str, id_field: &'static str) -> Result<ParsedValues, KeyValueMapError> {
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

    let id = values.remove(id_field).ok_or(KeyValueMapError::MissingId)?;

    Ok(ParsedValues { id, values })
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

macro_rules! setting_key {
    ($field:ident) => {
        stringify!($field)
    };
    ($field:ident => $key:literal) => {
        $key
    };
}

macro_rules! define_llm_arg {
    (
        field_name: $field_name:literal,
        struct $target:ident {
            $target_id:ident : $id_ty:ident {
                $( $variant:ident = $value:literal ),+ $(,)?
            },
            $( $field:ident : $field_ty:ty $(=> $map_key:literal)? ),+ $(,)?
        }
    ) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Display, EnumString)]
        pub enum $id_ty {
            $(
                #[strum(serialize = $value)]
                $variant,
            )+
        }

        impl IdValue for $id_ty {
            const ALLOWED_VALUES: &'static str = join_literals!($( $value ),+);
        }

        #[derive(Debug, Clone)]
        pub struct $target {
            pub $target_id: $id_ty,
            $( pub $field: $field_ty, )+
        }

        impl TryFrom<ParsedValues> for $target {
            type Error = LlmArgParseError;

            fn try_from(mut parsed: ParsedValues) -> Result<Self, Self::Error> {
                const SETTINGS: &[&str] = &[ $( setting_key!($field $(=> $map_key)?) ),+ ];
                let allowed_fields = allowed_fields_string($field_name, SETTINGS);
                let missing_settings = missing_settings_string(SETTINGS);

                let id_str = parsed.id;
                let $target_id = id_str
                    .parse::<$id_ty>()
                    .map_err(|_| LlmArgParseError::InvalidId {
                        kind: $field_name,
                        value: id_str,
                        allowed_values: <$id_ty as IdValue>::ALLOWED_VALUES,
                    })?;

                $(
                    let $field = parsed.values.remove(setting_key!($field $(=> $map_key)?));
                )+

                if let Some((unknown, _)) = parsed.values.into_iter().next() {
                    return Err(LlmArgParseError::UnknownField {
                        field: unknown,
                        allowed_fields,
                    });
                }

                if true $(&& $field.is_none())+ {
                    return Err(LlmArgParseError::MissingSetting {
                        kind: $field_name,
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
                const SETTINGS: &[&str] = &[ $( setting_key!($field $(=> $map_key)?) ),+ ];

                let values = parse_key_value_map(s, $field_name).map_err(|error| match error {
                    KeyValueMapError::MissingSeparator(part) => LlmArgParseError::UnknownField {
                        field: part,
                        allowed_fields: allowed_fields_string($field_name, SETTINGS),
                    },
                    KeyValueMapError::EmptyValue(name) => LlmArgParseError::EmptyValue(name),
                    KeyValueMapError::MissingId => LlmArgParseError::MissingId($field_name),
                })?;

                Self::try_from(values)
            }
        }
    };
}

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

define_llm_arg!(
    field_name: "service",
    struct LlmServiceArg {
        service: LlmServiceType {
            Openai = "openai",
            Gwdg = "gwdg",
            Kit = "kit",
        },
        key: Option<String>,
        default_model: Option<String> => "default-model",
    }
);

define_llm_arg!(
    field_name: "feature",
    struct LlmFeatureArg {
        feature: LlmFeatureType {
            Journaling = "journaling",
            Embedding = "embedding",
            Quiz = "quiz",
        }
        ,
        service: Option<String>,
        model: Option<String>,
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
