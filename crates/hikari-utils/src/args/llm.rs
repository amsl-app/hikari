use clap::Args;
use heck::ToKebabCase;
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
    fn allowed_values() -> String;
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

fn kebab_case(input: &str) -> String {
    input.to_kebab_case()
}

fn allowed_fields_string(id_field: &str, settings: &[String]) -> String {
    let mut fields = Vec::with_capacity(settings.len() + 1);
    fields.push(id_field.to_owned());
    fields.extend(settings.iter().cloned());
    fields.join(", ")
}

fn missing_settings_string(settings: &[String]) -> String {
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

macro_rules! define_llm_arg {
    (
        type: $target:ident,
        id_field: $target_id:ident : $id_ty:ident: {
            $( $variant:ident ),+ $(,)?
        },
        settings: [ $( $field:ident ),+ $(,)? ]
    ) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Display, EnumString)]
        #[strum(serialize_all = "kebab-case")]
        pub enum $id_ty {
            $( $variant, )+
        }

        impl IdValue for $id_ty {
            fn allowed_values() -> String {
                [$( kebab_case(stringify!($variant)) ),+].join(", ")
            }
        }

        #[derive(Debug, Clone)]
        pub struct $target {
            pub $target_id: $id_ty,
            $( pub $field: Option<String>, )+
        }

        impl TryFrom<ParsedValues> for $target {
            type Error = LlmArgParseError;

            fn try_from(mut parsed: ParsedValues) -> Result<Self, Self::Error> {
                let settings = vec![$( kebab_case(stringify!($field)) ),+];
                let allowed_fields = allowed_fields_string(stringify!($target_id), &settings);
                let missing_settings = missing_settings_string(&settings);

                let id_str = parsed.id;
                let $target_id = id_str
                    .parse::<$id_ty>()
                    .map_err(|_| LlmArgParseError::InvalidId {
                        kind: stringify!($target_id),
                        value: id_str,
                        allowed_values: <$id_ty as IdValue>::allowed_values(),
                    })?;

                $(
                    let $field = {
                        let key = kebab_case(stringify!($field));
                        parsed.values.remove(key.as_str())
                    };
                )+

                if let Some((unknown, _)) = parsed.values.into_iter().next() {
                    return Err(LlmArgParseError::UnknownField {
                        field: unknown,
                        allowed_fields,
                    });
                }

                if true $(&& $field.is_none())+ {
                    return Err(LlmArgParseError::MissingSetting {
                        kind: stringify!($target_id),
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
                let settings = vec![$( kebab_case(stringify!($field)) ),+];

                let values = parse_key_value_map(s, stringify!($target_id)).map_err(|error| match error {
                    KeyValueMapError::MissingSeparator(part) => LlmArgParseError::UnknownField {
                        field: part,
                        allowed_fields: allowed_fields_string(stringify!($target_id), &settings),
                    },
                    KeyValueMapError::EmptyValue(name) => LlmArgParseError::EmptyValue(name),
                    KeyValueMapError::MissingId => LlmArgParseError::MissingId(stringify!($target_id)),
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
        allowed_values: String,
    },
}

define_llm_arg!(
    type: LlmServiceArg,
    id_field: service: LlmServiceType: { Openai, Gwdg, Kit },
    settings: [key, default_model]
);

define_llm_arg!(
    type: LlmFeatureArg,
    id_field: feature: LlmFeatureType: { Journaling, Embedding, Quiz },
    settings: [service, model]
);

#[cfg(test)]
mod tests {
    use super::LlmServices;
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
        assert_eq!(cfg.service.to_string(), "kit");
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
        assert_eq!(cli.llm_services.llm_config[0].service.to_string(), "openai");
        assert_eq!(cli.llm_services.llm_config[1].service.to_string(), "gwdg");
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
        assert_eq!(cfg.feature.to_string(), "journaling");
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
        assert_eq!(cli.llm_services.llm_feature_config[0].feature.to_string(), "embedding");
        assert_eq!(cli.llm_services.llm_feature_config[1].feature.to_string(), "quiz");
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
