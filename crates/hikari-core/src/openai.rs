use crate::llm_config::LlmConfig;
use crate::openai::error::{FunctionCallError, OpenAiError};
use crate::openai::streaming::BoxedStream;
use crate::openai::tools::{Tool, ToolChoice};
use async_openai::Client;
use async_openai::config::OpenAIConfig;
use async_openai::types::{
    ChatCompletionMessageToolCall, ChatCompletionMessageToolCallChunk, ChatCompletionNamedToolChoice,
    ChatCompletionRequestMessage, ChatCompletionTool, ChatCompletionToolChoiceOption, ChatCompletionToolType,
    CreateChatCompletionRequestArgs, CreateChatCompletionResponse, FunctionCall, FunctionCallStream, FunctionName,
    FunctionObject,
};
use async_stream::try_stream;
use backoff::ExponentialBackoffBuilder;
use futures::StreamExt;
use serde_json::Value;
use std::error::Error;
use std::str::FromStr;
use std::time::Duration;
use tracing::instrument;
use typed_builder::TypedBuilder;

pub mod error;
pub mod streaming;
pub mod tools;

#[derive(Debug, Clone)]
pub struct Message {
    pub content: Content,
    pub tokens: Option<u32>,
}

impl TryFrom<CreateChatCompletionResponse> for Message {
    type Error = OpenAiError;

    fn try_from(value: CreateChatCompletionResponse) -> Result<Message, Self::Error> {
        let tokens = value.usage.map(|u| u.total_tokens);

        let first = value.choices.into_iter().next().ok_or(OpenAiError::EmptyResponse)?;

        if let Some(tool_calls) = first.message.tool_calls {
            let tool_calls: Vec<ToolCallResponse> = tool_calls
                .into_iter()
                .map(std::convert::TryInto::try_into)
                .collect::<Result<_, _>>()?;

            Ok(Message {
                content: Content::Tool(tool_calls),
                tokens,
            })
        } else if let Some(content) = first.message.content {
            Ok(Message {
                content: Content::Text(content.clone()),
                tokens,
            })
        } else {
            Err(OpenAiError::EmptyResponse)
        }
    }
}

#[derive(Debug, Clone)]
pub enum Content {
    Text(String),
    Tool(Vec<ToolCallResponse>),
}

#[derive(Debug, Clone)]
pub struct ToolCallResponse {
    pub name: String,
    pub arguments: Value,
}

impl TryFrom<ChatCompletionMessageToolCall> for ToolCallResponse {
    type Error = OpenAiError;

    fn try_from(value: ChatCompletionMessageToolCall) -> Result<Self, Self::Error> {
        let FunctionCall { name, arguments } = value.function;

        let arguments = Value::from_str(&arguments)?;

        Ok(ToolCallResponse { name, arguments })
    }
}

impl TryFrom<ChatCompletionMessageToolCallChunk> for ToolCallResponse {
    type Error = OpenAiError;

    fn try_from(value: ChatCompletionMessageToolCallChunk) -> Result<Self, Self::Error> {
        if let Some(FunctionCallStream {
            name: Some(name),
            arguments: Some(arguments),
        }) = value.function
        {
            let arguments = Value::from_str(&arguments)?;

            Ok(ToolCallResponse { name, arguments })
        } else {
            Err(OpenAiError::EmptyResponse)
        }
    }
}

impl Message {
    #[must_use]
    pub fn new(content: Content, tokens: Option<u32>) -> Self {
        Self { content, tokens }
    }
}

#[derive(TypedBuilder, Debug, Clone)]
pub struct CallConfig {
    total_timeout: Duration,
    iteration_timeout: Duration,
    #[builder(default = Duration::from_millis(100))]
    min_retry_interval: Duration,
    #[builder(default = Duration::from_secs(2))]
    max_retry_interval: Duration,
}

pub trait FunctionResponse: serde::de::DeserializeOwned {
    fn function_name() -> &'static str;
    fn function_description() -> &'static str;

    fn function_definition() -> serde_json::Value;

    fn fix_escapes(&mut self);
}

#[instrument(skip(llm_config, messages))]
pub async fn openai_call_function_with_timeout<T: FunctionResponse>(
    llm_config: &LlmConfig,
    config: CallConfig,
    messages: Vec<ChatCompletionRequestMessage>,
) -> Result<T, OpenAiError> {
    let name = T::function_name();

    let request = CreateChatCompletionRequestArgs::default()
        .model(llm_config.get_journaling_model())
        .messages(messages)
        .max_tokens(1024u16)
        .tools(vec![ChatCompletionTool {
            r#type: ChatCompletionToolType::Function,
            function: FunctionObject {
                name: name.to_string(),
                description: Some(T::function_description().to_string()),
                parameters: Some(T::function_definition()),
                strict: None,
            },
        }])
        .tool_choice(ChatCompletionToolChoiceOption::Named(ChatCompletionNamedToolChoice {
            r#type: ChatCompletionToolType::Function,
            function: FunctionName { name: name.to_string() },
        }))
        .build()?;

    let http_client = reqwest::Client::builder()
        .timeout(config.iteration_timeout)
        .build()
        .map_err(|error| {
            tracing::error!(error = &error as &dyn Error, "failed to build http client for openai");
            OpenAiError::HttpClientBuild(error)
        })?;

    let mut backoff_builder = ExponentialBackoffBuilder::default();
    backoff_builder
        .with_max_interval(config.max_retry_interval)
        .with_initial_interval(config.min_retry_interval)
        .with_max_elapsed_time(Some(config.total_timeout));

    let backoff = backoff_builder.build();

    let client = Client::with_config(llm_config.get_journaling_openai_config())
        .with_http_client(http_client)
        .with_backoff(backoff);

    tracing::debug!("sending openai request");
    let res = client.chat().create(request).await;
    let chat_completion = res.map_err(|error| {
        tracing::warn!(error = &error as &dyn Error, "open AI call failed");
        OpenAiError::Api(error)
    })?;

    check_function_call(&chat_completion)
}

#[instrument(skip_all)]
fn check_function_call<T: FunctionResponse>(chat_completion: &CreateChatCompletionResponse) -> Result<T, OpenAiError> {
    let choice = chat_completion.choices.first().ok_or(OpenAiError::EmptyResponse)?;
    let message = &choice.message;

    let function_call = message
        .tool_calls
        .as_ref()
        .ok_or(FunctionCallError::Missing)?
        .first()
        .ok_or(FunctionCallError::Missing)?;

    if function_call.function.name != T::function_name() {
        tracing::warn!(
            expected_function = T::function_name(),
            called_function = &function_call.function.name,
            "assistant tried to call the wrong function"
        );
        return Err(FunctionCallError::WrongFunction.into());
    }

    let mut res: T = serde_json::from_str(&function_call.function.arguments).map_err(|error| {
        tracing::warn!(
            erorr = &error as &dyn Error,
            arguments = function_call.function.arguments,
            "failed to parse function call arguments"
        );
        FunctionCallError::InvalidSyntax
    })?;
    res.fix_escapes();
    Ok(res)
}

pub enum OpenAiCallResult {
    Message(Message),
    Stream(BoxedStream),
}

#[allow(clippy::too_many_arguments)]
pub async fn openai_call_with_timeout(
    config: CallConfig,
    openai_config: OpenAIConfig,
    streaming: bool,
    temperature: Option<f32>,
    model: &str,
    messages: Vec<ChatCompletionRequestMessage>,
    tools: Vec<Box<dyn Tool>>,
    tool_choice: Option<ToolChoice>,
) -> Result<OpenAiCallResult, OpenAiError> {
    let mut request = CreateChatCompletionRequestArgs::default();
    request.model(model).messages(messages);

    if let Some(temperature) = temperature {
        request.temperature(temperature);
    }

    let tools_defs = tools
        .iter()
        .map(|tool| tool.as_openai_tool())
        .collect::<Result<Vec<ChatCompletionTool>, OpenAiError>>()?;

    request.tools(tools_defs);

    if let Some(tool_choice) = tool_choice {
        request.tool_choice(tool_choice);
    }

    tracing::debug!(?request, "OpenAI request");

    let request = request.build()?;

    let http_client = reqwest::Client::builder()
        .timeout(config.total_timeout)
        .build()
        .map_err(|error| {
            tracing::error!(error = &error as &dyn Error, "failed to build http client for openai");
            OpenAiError::HttpClientBuild(error)
        })?;

    let mut backoff_builder = ExponentialBackoffBuilder::default();
    backoff_builder.with_max_interval(config.max_retry_interval);

    let backoff = backoff_builder.build();

    let client = Client::with_config(openai_config)
        .with_http_client(http_client)
        .with_backoff(backoff);

    if streaming {
        tracing::debug!("Using streaming OpenAI call");
        let res = client.chat().create_stream(request).await;
        match res {
            Ok(mut stream) => {
                let stream = try_stream! {
                    while let Some(chunk) = stream.next().await {
                        let chunk = chunk?;
                        let tokens = chunk.usage.map(|u| u.total_tokens);

                        let first = chunk.choices.into_iter().next();

                        if let Some(first) = first {
                            if let Some(tool_calls) = first.delta.tool_calls {
                                let tool_calls: Vec<ToolCallResponse> =
                                    tool_calls.into_iter().map(std::convert::TryInto::try_into).collect::<Result<_, _>>()?;

                                yield Message {
                                    content: Content::Tool(tool_calls),
                                    tokens,
                                }
                            } else if let Some(content) = first.delta.content {
                                yield Message {
                                    content: Content::Text(content.clone()),
                                    tokens,
                                }
                            }
                        }
                    }
                }
                .boxed();
                Ok(OpenAiCallResult::Stream(stream))
            }
            Err(error) => Err(OpenAiError::Api(error)),
        }
    } else {
        let res = client.chat().create(request).await;
        let chat_completion = res.map_err(|error| {
            tracing::warn!(error = &error as &dyn Error, "open AI call failed");
            OpenAiError::Api(error)
        })?;

        let message: Message = chat_completion.try_into()?;
        Ok(OpenAiCallResult::Message(message))
    }
}
