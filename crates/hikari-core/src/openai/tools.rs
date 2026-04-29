use crate::openai::error::OpenAiError;
use async_openai::types::chat::{
    ChatCompletionNamedToolChoice, ChatCompletionTool, ChatCompletionToolChoiceOption, ChatCompletionTools,
    FunctionName, FunctionObject, ToolChoiceOptions,
};
use schemars::Schema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::borrow::Cow;
use std::collections::HashMap;
use std::hash::Hash;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ToolChoice {
    Auto,
    Named(String),
    // Required, Since this is not always supported, we encourage to use Auto or Single Tools with enum variants instead.
}

impl From<ToolChoice> for ChatCompletionToolChoiceOption {
    fn from(choice: ToolChoice) -> Self {
        match choice {
            ToolChoice::Auto => ChatCompletionToolChoiceOption::Mode(ToolChoiceOptions::Auto),
            ToolChoice::Named(name) => ChatCompletionToolChoiceOption::Function(ChatCompletionNamedToolChoice {
                function: FunctionName { name: name.clone() },
            }),
            // ToolChoice::Required => ChatCompletionToolChoiceOption::Mode(ToolChoiceOptions::Required),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ToolSchema(pub Schema);

impl ToolSchema {
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.0.get("title").and_then(|v| v.as_str())
    }
}

impl From<Schema> for ToolSchema {
    fn from(schema: Schema) -> Self {
        ToolSchema(schema)
    }
}

impl TryFrom<ToolSchema> for ChatCompletionTools {
    type Error = OpenAiError;

    fn try_from(schema: ToolSchema) -> Result<ChatCompletionTools, Self::Error> {
        let mut properties = schema
            .0
            .as_object()
            .cloned()
            .ok_or(OpenAiError::ToolError("Schema is not an object".to_string()))?;

        let title = properties
            .remove("title")
            .and_then(|t| t.as_str().map(std::string::ToString::to_string))
            .ok_or(OpenAiError::ToolError("Missing title in schema".to_string()))?;

        let description = properties
            .remove("description")
            .and_then(|d| d.as_str().map(std::string::ToString::to_string))
            .ok_or(OpenAiError::ToolError("Missing description in schema".to_string()))?;

        let function = FunctionObject {
            name: title.clone(),
            description: Some(description.clone()),
            parameters: Some(Value::Object(properties)),
            strict: None,
        };

        Ok(ChatCompletionTools::Function(ChatCompletionTool { function }))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiField<'a> {
    pub r#type: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<Cow<'a, str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#enum: Option<Vec<&'a str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Cow<'a, str>>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub properties: HashMap<&'a str, OpenApiField<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<OpenApiField<'a>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_items: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_items: Option<usize>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub required: Vec<&'a str>,
}

impl<'a> From<OpenApiField<'a>> for ToolSchema {
    fn from(field: OpenApiField<'a>) -> Self {
        let value = serde_json::to_value(field).expect("Serialization failed that should not fail");
        ToolSchema(serde_json::from_value(value).expect("Deserialization failed that should not fail"))
    }
}

pub trait AsOpenApiField<'a> {
    fn openapi_field(&'a self) -> OpenApiField<'a>;
}

impl<'a> OpenApiField<'a> {
    #[must_use]
    pub fn new(r#type: &'a str) -> Self {
        OpenApiField {
            r#type,
            title: None,
            r#enum: None,
            description: None,
            properties: HashMap::new(),
            items: None,
            min_items: None,
            max_items: None,
            required: vec![],
        }
    }

    #[must_use]
    pub fn object() -> Self {
        OpenApiField::new("object")
    }

    #[must_use]
    pub fn title<D: Into<Cow<'a, str>>>(mut self, title: D) -> Self {
        self.title = Some(title.into());
        self
    }

    #[must_use]
    pub fn description<D: Into<Cow<'a, str>>>(mut self, description: D) -> Self {
        self.description = Some(description.into());
        self
    }

    #[must_use]
    pub fn properties<I: Into<HashMap<&'a str, OpenApiField<'a>>>>(mut self, properties: I) -> Self {
        self.properties = properties.into();
        self
    }

    #[must_use]
    pub fn items(mut self, items: OpenApiField<'a>) -> Self {
        self.items = Some(Box::new(items));
        self
    }

    #[must_use]
    pub fn min_items(mut self, min_items: usize) -> Self {
        self.min_items = Some(min_items);
        self
    }

    #[must_use]
    pub fn max_items(mut self, max_items: usize) -> Self {
        self.max_items = Some(max_items);
        self
    }

    #[must_use]
    pub fn required<I: Into<Vec<&'a str>>>(mut self, required: I) -> Self {
        self.required = required.into();
        self
    }

    #[must_use]
    pub fn r#enum<I: Into<Vec<&'a str>>>(mut self, r#enum: I) -> Self {
        self.r#enum = Some(r#enum.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};

    #[test]
    fn test_as_openai_tool() {
        #[derive(JsonSchema, Serialize, Deserialize)]
        #[schemars(description = "An struct within an enum tool for demonstration purposes", inline)]
        pub struct EnumStruct {
            pub field: String,
        }

        #[derive(JsonSchema, Serialize, Deserialize)]
        #[schemars(description = "An enum tool for demonstration purposes", inline)]
        #[serde(untagged)]
        enum MyToolEnum {
            /// Variant A of the test tool
            VariantA(EnumStruct),
            /// Another variant of the test tool
            VariantB {
                /// A field in VariantB
                field_b: String,
            },
            VariantC,
        }

        #[derive(JsonSchema, Serialize, Deserialize)]
        #[schemars(description = "A test tool for demonstration purposes")]
        struct MyTestTool {
            /// The name of the person
            name: String,
            /// Optional age
            age: Option<u32>,
            /// An enum field for demonstration
            enum_field: MyToolEnum,
        }

        let schema = ToolSchema(schemars::schema_for!(MyTestTool));

        let openai_tool: ChatCompletionTools = schema.try_into().expect("Failed to convert to OpenAI tool");

        match openai_tool {
            ChatCompletionTools::Function(f) => {
                assert_eq!(f.function.name, "MyTestTool");
                assert_eq!(
                    f.function.description.as_deref(),
                    Some("A test tool for demonstration purposes")
                );
                let params = f.function.parameters.as_ref().expect("Parameters should exist");
                assert_eq!(params["type"], "object");
                assert_eq!(params["properties"]["name"]["type"], "string");
                assert_eq!(params["properties"]["name"]["description"], "The name of the person");
                assert_eq!(params["properties"]["age"]["description"], "Optional age");
                assert_eq!(
                    params["properties"]["enum_field"]["description"],
                    "An enum field for demonstration"
                );
            }
            _ => panic!("Expected Function tool"),
        }
    }

    #[test]
    fn test_manual_schema() {
        let schema = OpenApiField::object()
            .title("MyManualTool")
            .description("A manually defined tool schema")
            .properties(
                vec![
                    ("field1", OpenApiField::new("string").description("A string field")),
                    ("field2", OpenApiField::new("number").description("A number field")),
                ]
                .into_iter()
                .collect::<HashMap<&str, OpenApiField>>(),
            );
        let tool_schema: ToolSchema = schema.into();
        let openai_tool: ChatCompletionTools = tool_schema.try_into().expect("Failed to convert to OpenAI tool");
        match openai_tool {
            ChatCompletionTools::Function(f) => {
                assert_eq!(f.function.name, "MyManualTool");
                assert_eq!(
                    f.function.description.as_deref(),
                    Some("A manually defined tool schema")
                );
                let params = f.function.parameters.as_ref().expect("Parameters should exist");
                assert_eq!(params["type"], "object");
                assert_eq!(params["properties"]["field1"]["type"], "string");
                assert_eq!(params["properties"]["field1"]["description"], "A string field");
                assert_eq!(params["properties"]["field2"]["type"], "number");
                assert_eq!(params["properties"]["field2"]["description"], "A number field");
            }
            _ => panic!("Expected Function tool"),
        }
    }
}
