use crate::openai::error::OpenAiError;
use crate::openai::streaming::MessageStream;
use crate::openai::tools::{ToolChoice, ToolSchema};
use async_openai::Client;
use async_openai::config::OpenAIConfig;
use async_openai::types::chat::{
    ChatCompletionMessageToolCall, ChatCompletionMessageToolCallChunk, ChatCompletionMessageToolCalls,
    ChatCompletionRequestMessage, ChatCompletionTools, CreateChatCompletionRequestArgs, CreateChatCompletionResponse,
    FunctionCall, FunctionCallStream,
};
use async_stream::try_stream;
use backoff::ExponentialBackoffBuilder;
use futures::StreamExt;
use regex::Regex;
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::error::Error;
use std::str::FromStr;
use std::sync::LazyLock;
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

impl Message {
    #[must_use]
    pub fn new(content: Content, tokens: Option<u32>) -> Self {
        Self { content, tokens }
    }
}

static THINKING_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<think>(.*?)</think>").expect("thinking regex is invalid"));

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
            let thinking = THINKING_RE
                .captures(&content)
                .and_then(|caps| caps.get(1).map(|t| t.as_str().trim().to_string()));

            let text = THINKING_RE.replace_all(&content, "").to_string();

            let text = match text.trim().is_empty() {
                true => None,
                false => Some(text),
            };

            tracing::debug!(content = &content, thinking = ?thinking, "cleaned message content");

            Ok(Message {
                content: Content::Text { text, thinking },
                tokens,
            })
        } else {
            Err(OpenAiError::EmptyResponse)
        }
    }
}

#[derive(Debug, Clone)]
pub enum Content {
    Text {
        text: Option<String>,
        thinking: Option<String>,
    },
    Tool(Vec<ToolCallResponse>),
}

#[derive(Debug, Clone)]
pub struct ToolCallResponse {
    pub name: String,
    pub thinking: Option<String>,
    pub arguments: Value,
}

impl TryFrom<ChatCompletionMessageToolCall> for ToolCallResponse {
    type Error = OpenAiError;

    fn try_from(value: ChatCompletionMessageToolCall) -> Result<Self, Self::Error> {
        let FunctionCall { name, arguments } = value.function;
        let thinking = THINKING_RE
            .captures(&arguments)
            .and_then(|caps| caps.get(1).map(|t| t.as_str().trim().to_string()));
        let arguments = THINKING_RE.replace_all(&arguments, "").to_string();
        tracing::debug!(arguments = &arguments, "cleaned function call arguments");

        let arguments = Value::from_str(&arguments)?;

        Ok(ToolCallResponse {
            name,
            arguments,
            thinking,
        })
    }
}

impl TryFrom<ChatCompletionMessageToolCalls> for ToolCallResponse {
    type Error = OpenAiError;

    fn try_from(value: ChatCompletionMessageToolCalls) -> Result<Self, Self::Error> {
        match value {
            ChatCompletionMessageToolCalls::Function(tool_call) => tool_call.try_into(),
            ChatCompletionMessageToolCalls::Custom(_) => Err(OpenAiError::EmptyResponse),
        }
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
            let thinking = THINKING_RE
                .captures(&arguments)
                .and_then(|caps| caps.get(1).map(|t| t.as_str().trim().to_string()));
            let arguments = THINKING_RE.replace_all(&arguments, "").to_string();
            tracing::debug!(arguments = &arguments, "cleaned function call arguments");

            let arguments = Value::from_str(&arguments)?;

            Ok(ToolCallResponse {
                name,
                arguments,
                thinking,
            })
        } else {
            Err(OpenAiError::EmptyResponse)
        }
    }
}

#[derive(TypedBuilder, Debug, Clone)]
pub struct CallConfig {
    #[builder(default = Duration::from_secs(30))]
    total_timeout: Duration,
    #[builder(default = Duration::from_secs(10))]
    iteration_timeout: Duration,
    #[builder(default = Duration::from_millis(500))]
    min_retry_interval: Duration,
    #[builder(default = Duration::from_secs(1))]
    max_retry_interval: Duration,
}

pub enum OpenAiCallResult {
    Message(Message),
    Stream(MessageStream),
}

#[allow(clippy::too_many_arguments)]
#[instrument(skip(openai_config, messages, tools, tool_choice))]
pub async fn openai_call_with_timeout(
    config: CallConfig,
    openai_config: OpenAIConfig,
    streaming: bool,
    temperature: Option<f32>,
    model: &str,
    messages: Vec<ChatCompletionRequestMessage>,
    tools: Vec<ToolSchema>,
    tool_choice: Option<ToolChoice>,
) -> Result<OpenAiCallResult, OpenAiError> {
    let mut request = CreateChatCompletionRequestArgs::default();
    request.model(model).messages(messages);

    if let Some(temperature) = temperature {
        request.temperature(temperature);
    }

    let tools = tools
        .into_iter()
        .map(|tool| tool.try_into())
        .collect::<Result<Vec<ChatCompletionTools>, OpenAiError>>()?;

    request.tools(tools);

    if let Some(tool_choice) = tool_choice {
        request.tool_choice(tool_choice);
    }

    tracing::debug!(?request, "OpenAI request");

    let request = request.build()?;

    let http_client = reqwest::Client::builder()
        .timeout(config.iteration_timeout)
        .build()
        .map_err(|error| {
            tracing::error!(error = &error as &dyn Error, "failed to build http client for openai");
            OpenAiError::HttpClientBuild(error)
        })?;

    let mut backoff_builder = ExponentialBackoffBuilder::default();
    backoff_builder
        .with_initial_interval(config.min_retry_interval)
        .with_max_interval(config.max_retry_interval)
        .with_max_elapsed_time(Some(config.total_timeout));

    let backoff = backoff_builder.build();

    let client = Client::with_config(openai_config)
        .with_http_client(http_client)
        .with_backoff(backoff);

    if streaming {
        tracing::debug!("Using streaming OpenAI call");
        let res = client.chat().create_stream(request).await;
        match res {
            Ok(mut stream) => {
                let mut in_think_block = false;
                let mut think_buffer = String::new();
                let mut text_buffer = String::new();

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
                                // If we previously detected a <think> tag, we are currently in a think block and all content should be treated as thinking until we find the closing tag. 
                                // This is necessary to properly handle cases where the <think> tag is split across multiple chunks.
                                if in_think_block {
                                    let already_sent = think_buffer.len();
                                    think_buffer.push_str(&content); // To always check the whole buffer for the closing tag, even if it spans multiple chunks

                                    if let Some(pos) = think_buffer.find("</think>") {
                                        in_think_block = false;
                                        let thinking = think_buffer[already_sent..pos].to_string();
                                        let text = think_buffer[pos + 8..].to_string();

                                        think_buffer.clear();
                                        text_buffer.push_str(&text);

                                        let thinking: Option<String> = match thinking.trim().is_empty() {
                                            true => None,
                                            false => Some(thinking),
                                        };

                                        let text: Option<String> = match text.trim().is_empty() {
                                            true => None,
                                            false => Some(text),
                                        };

                                        yield Message {
                                            content: Content::Text{
                                                text,
                                                thinking
                                            },
                                            tokens: None,
                                        }
                                    } else {
                                        yield Message {
                                            content: Content::Text{
                                                text: None,
                                                thinking: Some(content),
                                            },
                                            tokens,
                                        }
                                    }
                                    continue; // Don't process the content further, since it's part of the think block and we will handle it once we find the closing tag
                                }
                                // If we are not in the think block
                                let already_sent = text_buffer.len();
                                text_buffer.push_str(&content);
                                if let Some(pos) = text_buffer.find("<think>") {
                                    in_think_block = true;

                                    let text = text_buffer[already_sent..pos].to_string();
                                    let thinking = text_buffer[pos + 7..].to_string();

                                    text_buffer.clear();
                                    think_buffer.push_str(&thinking);

                                    let text: Option<String> = match text.trim().is_empty() {
                                        true => None,
                                        false => Some(text),
                                    };
                                    let thinking: Option<String> = match thinking.trim().is_empty() {
                                        true => None,
                                        false => Some(thinking),
                                    };

                                    yield Message {
                                        content: Content::Text{
                                            text,
                                            thinking,
                                        },
                                        tokens: None,
                                    }
                                } else {
                                    yield Message {
                                        content: Content::Text{
                                            text: Some(content),
                                            thinking: None,
                                        },
                                        tokens,
                                    }
                                }
                            }
                        }
                    }
                }
                .boxed();
                Ok(OpenAiCallResult::Stream(MessageStream::new(stream)))
            }
            Err(error) => Err(OpenAiError::Api(error)),
        }
    } else {
        let res: Result<CreateChatCompletionResponse, async_openai::error::OpenAIError> =
            client.chat().create(request).await;
        tracing::debug!(?res, "received OpenAI response");
        let chat_completion = res.map_err(|error| {
            tracing::warn!(error = &error as &dyn Error, "open AI call failed");
            OpenAiError::Api(error)
        })?;

        let message: Message = chat_completion.try_into()?;
        Ok(OpenAiCallResult::Message(message))
    }
}

#[instrument(skip(openai_config, messages))]
pub async fn openai_single_tool_call<T: DeserializeOwned + JsonSchema>(
    config: CallConfig,
    openai_config: OpenAIConfig,
    temperature: Option<f32>,
    model: &str,
    messages: Vec<ChatCompletionRequestMessage>,
) -> Result<(T, Option<u32>), OpenAiError> {
    let schema = schemars::schema_for!(T);
    let tool_schema: ToolSchema = schema.into();
    let tool_name = tool_schema
        .name()
        .ok_or_else(|| OpenAiError::ToolError("Schema missing title/name".to_string()))?
        .to_string();

    let llm_response = openai_call_with_timeout(
        config,
        openai_config,
        false,
        temperature,
        model,
        messages,
        vec![tool_schema],
        Some(ToolChoice::Named(tool_name)),
    )
    .await?;

    let llm_response = match llm_response {
        OpenAiCallResult::Stream(_) => Err(OpenAiError::UnexpectedResponseFormat),
        OpenAiCallResult::Message(msg) => Ok(msg),
    }?;

    let response = match llm_response.content {
        Content::Tool(tool_calls) => tool_calls
            .into_iter()
            .next()
            .ok_or(OpenAiError::UnexpectedResponseFormat),
        Content::Text { .. } => Err(OpenAiError::UnexpectedResponseFormat),
    }?;

    let res: T = serde_json::from_value(response.arguments)?;
    Ok((res, llm_response.tokens))
}
