use async_openai::types::{
    ChatCompletionNamedToolChoice, ChatCompletionTool, ChatCompletionToolArgs, ChatCompletionToolChoiceOption,
    ChatCompletionToolType, FunctionName, FunctionObjectArgs,
};
use async_trait::async_trait;
use serde_json::Value;

use crate::openai::error::OpenAiError;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ToolChoice {
    Auto,
    Named(String),
    Required,
}

impl From<ToolChoice> for ChatCompletionToolChoiceOption {
    fn from(choice: ToolChoice) -> Self {
        match choice {
            ToolChoice::Auto => ChatCompletionToolChoiceOption::Auto,
            ToolChoice::Named(name) => ChatCompletionToolChoiceOption::Named(ChatCompletionNamedToolChoice {
                r#type: ChatCompletionToolType::Function,
                function: FunctionName { name: name.clone() },
            }),
            ToolChoice::Required => ChatCompletionToolChoiceOption::Required,
        }
    }
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> Value;

    fn as_openai_tool(&self) -> Result<ChatCompletionTool, OpenAiError> {
        let call = FunctionObjectArgs::default()
            .name(self.name().to_string())
            .description(self.description().to_string())
            .parameters(self.parameters())
            .strict(false)
            .build()?;

        let res = ChatCompletionToolArgs::default().function(call).build()?;
        Ok(res)
    }
}

use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiField<'a> {
    pub r#type: &'a str,
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

pub trait AsOpenApiField<'a> {
    fn openapi_field(&'a self) -> OpenApiField<'a>;
}

impl<'a> OpenApiField<'a> {
    #[must_use]
    pub fn new(r#type: &'a str) -> Self {
        OpenApiField {
            r#type,
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
