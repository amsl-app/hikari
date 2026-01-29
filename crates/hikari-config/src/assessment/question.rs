use crate::assessment::error::ValidationError;
use heck::ToSnakeCase;
use hikari_utils::id_map::ItemId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use strum::IntoStaticStr;
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema, JsonSchema)]
#[serde(untagged)]
pub enum Answer {
    Scale(u8),
    Bool(bool),
    Text(String),
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Question {
    /// # Unique identifier for the question
    /// This ID is used to reference the question within the assessment.
    pub id: String,
    /// # Title of the question
    /// A human-readable title for the question.
    pub title: String,
    #[serde(flatten)]
    /// # Body of the question
    /// Contains the detailed content and type of the question.
    pub body: QuestionBody,
    #[serde(skip_serializing)]
    pub answer: Option<Answer>,
}

impl ItemId for Question {
    type IdType = String;

    fn id(&self) -> Self::IdType {
        self.id.clone()
    }
}

pub trait QuestionExt {
    fn validate(&self, value: &AnswerValue) -> Result<(), ValidationError>;
}

macro_rules! get_answer_value {
    ($value:ident, $variant:ident) => {{
        let AnswerValue::$variant { value } = $value else {
            let ty: &'static str = $value.into();
            let expected_type: &'static str = stringify!($variant);
            let expected_type = expected_type.to_snake_case();
            return Err(ValidationError::InvalidAnswerType {
                expected_type,
                actual_type: ty.to_string(),
            });
        };
        value
    }};
}

impl QuestionExt for Question {
    fn validate(&self, value: &AnswerValue) -> Result<(), ValidationError> {
        match &self.body {
            QuestionBody::Scale(q) => {
                let value = *get_answer_value!(value, SmallInt);
                if value < q.min || value > q.max {
                    return Err(ValidationError::AnswerOutOfRange {
                        min: q.min,
                        max: q.max,
                        value,
                    });
                }
            }
            QuestionBody::Textfield(_) | QuestionBody::Textarea(_) => {
                get_answer_value!(value, Text);
            }
            QuestionBody::Select(_) | QuestionBody::SingleChoice(_) => {
                get_answer_value!(value, Bool);
            }
            QuestionBody::MultiChoice(q) => {
                let value = get_answer_value!(value, Text);
                if !q.options.contains(value) {
                    return Err(ValidationError::InvalidOption {
                        value: value.to_owned(),
                    });
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema, strum::Display, strum::IntoStaticStr)]
#[schema(example = json!({"value": true}))]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
#[strum(serialize_all = "snake_case")]
pub enum AnswerValue {
    Bool { value: bool },
    Text { value: String },
    SmallInt { value: u8 },
}

#[derive(Serialize, Deserialize, Debug, Clone, IntoStaticStr, ToSchema, JsonSchema)]
#[serde(tag = "type", content = "body")]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
#[strum(serialize_all = "snake_case")]
pub enum QuestionBody {
    Scale(LikertScaleBody),
    Textfield(TextBody),
    Textarea(TextBody),
    Select(SelectBody),
    SingleChoice(EmptyBody),
    MultiChoice(MultiBody),
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct SelectBody {
    pub yes: Option<String>,
    pub no: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct LikertScaleBody {
    pub min: u8,
    pub max: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint_min: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint_max: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct TextBody {
    pub placeholder: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct MultiBody {
    pub options: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
// TODO remove after frontend handles this better
pub struct EmptyBody {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expected_type() {
        let q = Question {
            id: String::new(),
            title: String::new(),
            body: QuestionBody::Scale(LikertScaleBody {
                min: 1,
                max: 5,
                hint_min: None,
                hint_max: None,
            }),
            answer: None,
        };
        let value = AnswerValue::Bool { value: true };
        let Err(ValidationError::InvalidAnswerType {
            expected_type,
            actual_type,
        }) = q.validate(&value)
        else {
            panic!("expected InvalidAnswerType error");
        };
        assert_eq!(expected_type, "small_int");
        assert_eq!(actual_type, "bool");
        let value = AnswerValue::SmallInt { value: 3 };
        assert!(q.validate(&value).is_ok());
    }
}
