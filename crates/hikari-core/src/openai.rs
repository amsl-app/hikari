use crate::openai::error::OpenAiError;
use crate::openai::streaming::MessageStream;
use crate::openai::tools::{ToolChoice, ToolSchema};
use async_openai::Client;
use async_openai::config::{Config, OpenAIConfig};
use async_openai::types::chat::{
    ChatCompletionMessageToolCall, ChatCompletionMessageToolCallChunk, ChatCompletionMessageToolCalls,
    ChatCompletionRequestMessage, ChatCompletionTools, CreateChatCompletionRequestArgs, CreateChatCompletionResponse,
    FunctionCall, FunctionCallStream,
};
use async_stream::try_stream;
use backoff::ExponentialBackoffBuilder;
use futures::Stream;
use futures::StreamExt;
use regex::Regex;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::error::Error;
use std::fmt::Display;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::LazyLock;
use std::time::Duration;
use tokio::time::Instant;
use tracing::instrument;
use typed_builder::TypedBuilder;

pub mod error;
pub mod streaming;
pub mod tools;

#[derive(Deserialize, Debug, Clone, Copy, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum ReasoningEffort {
    None,
    Minimal,
    Low,
    Medium,
    High,
    Xhigh,
}

impl Display for ReasoningEffort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            ReasoningEffort::None => "none",
            ReasoningEffort::Minimal => "minimal",
            ReasoningEffort::Low => "low",
            ReasoningEffort::Medium => "medium",
            ReasoningEffort::High => "high",
            ReasoningEffort::Xhigh => "xhigh",
        };
        write!(f, "{}", label)
    }
}

impl From<ReasoningEffort> for async_openai::types::chat::ReasoningEffort {
    fn from(value: ReasoningEffort) -> Self {
        match value {
            ReasoningEffort::None => async_openai::types::chat::ReasoningEffort::None,
            ReasoningEffort::Minimal => async_openai::types::chat::ReasoningEffort::Minimal,
            ReasoningEffort::Low => async_openai::types::chat::ReasoningEffort::Low,
            ReasoningEffort::Medium => async_openai::types::chat::ReasoningEffort::Medium,
            ReasoningEffort::High => async_openai::types::chat::ReasoningEffort::High,
            ReasoningEffort::Xhigh => async_openai::types::chat::ReasoningEffort::Xhigh,
        }
    }
}

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

            let content_len = content.len();
            let thinking_len = thinking.as_ref().map(std::string::String::len);
            let text_len = text.as_ref().map(std::string::String::len);

            tracing::debug!(
                content_len,
                has_thinking = thinking.is_some(),
                thinking_len,
                has_text = text.is_some(),
                text_len,
                "cleaned message content"
            );
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
        let arguments_len = arguments.len();
        let arguments = Value::from_str(&arguments)?;
        let argument_keys: Vec<&str> = match &arguments {
            Value::Object(map) => map.keys().map(String::as_str).collect(),
            _ => Vec::new(),
        };
        tracing::debug!(
            arguments_len,
            argument_keys = ?argument_keys,
            "cleaned function call arguments"
        );

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
#[instrument(skip(
    config,
    openai_config,
    temperature,
    reasoning_effort,
    model,
    messages,
    tools,
    tool_choice
))]
pub async fn openai_call_with_timeout(
    config: CallConfig,
    openai_config: OpenAIConfig,
    streaming: bool,
    temperature: Option<f32>,
    reasoning_effort: Option<ReasoningEffort>,
    model: &str,
    messages: Vec<ChatCompletionRequestMessage>,
    tools: Vec<ToolSchema>,
    tool_choice: Option<ToolChoice>,
) -> Result<OpenAiCallResult, OpenAiError> {
    let start_time = Instant::now();
    let model_label = model.to_string();
    let service = openai_config.api_base().to_string();

    let mut request = CreateChatCompletionRequestArgs::default();
    request.model(model).messages(messages);

    if let Some(temperature) = temperature {
        request.temperature(temperature);
    }

    if let Some(reasoning_effort) = reasoning_effort {
        request.reasoning_effort(reasoning_effort);
    }

    let tools = tools
        .into_iter()
        .map(|tool| tool.try_into())
        .collect::<Result<Vec<ChatCompletionTools>, OpenAiError>>()?;

    if !tools.is_empty() {
        tracing::debug!(tool_count = tools.len(), "adding tools to OpenAI request");
        request.tools(tools);

        if let Some(tool_choice) = tool_choice {
            request.tool_choice(tool_choice);
        }
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
            Ok(stream) => {
                let stream = process_stream(stream, start_time, service, model_label);
                Ok(OpenAiCallResult::Stream(MessageStream::new(stream)))
            }
            Err(error) => Err(OpenAiError::Api(error)),
        }
    } else {
        let res: Result<CreateChatCompletionResponse, async_openai::error::OpenAIError> =
            client.chat().create(request).await;

        let elapsed = start_time.elapsed();
        metrics::histogram!(
            "llm_time_to_last_token_ms",
            "service" => service,
            "model" => model_label,
        )
        .record(elapsed.as_millis() as f64);

        tracing::debug!(?res, "received OpenAI response");
        let chat_completion = res.map_err(|error| {
            tracing::warn!(error = &error as &dyn Error, "open AI call failed");
            OpenAiError::Api(error)
        })?;

        let message: Message = chat_completion.try_into()?;
        Ok(OpenAiCallResult::Message(message))
    }
}

pub(crate) fn process_stream(
    mut stream: impl Stream<
        Item = Result<async_openai::types::chat::CreateChatCompletionStreamResponse, async_openai::error::OpenAIError>,
    > + Unpin
    + Send
    + 'static,
    start_time: Instant,
    service: String,
    model: String,
) -> Pin<Box<dyn Stream<Item = Result<Message, crate::openai::error::StreamingError>> + Send>> {
    let mut in_think_block = false;
    let mut buffer = String::new();

    try_stream! {
        let mut first_token_received = false;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            let tokens = chunk.usage.map(|u| u.total_tokens);

            let first = chunk.choices.into_iter().next();

            if let Some(first) = first {
                if !first_token_received
                    && (first.delta.content.is_some() || first.delta.tool_calls.is_some()) {
                        first_token_received = true;
                        metrics::histogram!(
                            "llm_time_to_first_token_ms",
                            "service" => service.clone(),
                            "model" => model.clone(),
                        ).record(start_time.elapsed().as_millis() as f64);
                    }
                if let Some(tool_calls) = first.delta.tool_calls {
                    let tool_calls: Vec<ToolCallResponse> =
                        tool_calls.into_iter().map(std::convert::TryInto::try_into).collect::<Result<_, _>>()?;

                    yield Message {
                        content: Content::Tool(tool_calls),
                        tokens,
                    }
                } else if let Some(content) = first.delta.content {
                    buffer.push_str(&content);

                    loop {
                        if !in_think_block {
                            if let Some(pos) = buffer.find("<think>") {
                                let text = buffer[..pos].to_string();
                                buffer.drain(..pos + 7);
                                in_think_block = true;

                                let text = match text.trim().is_empty() {
                                    true => None,
                                    false => Some(text),
                                };

                                if text.is_some() {
                                    yield Message {
                                        content: Content::Text { text, thinking: None },
                                        tokens: None,
                                    }
                                }
                            } else {
                                // No <think> tag found.
                                // We can yield everything up to the last '<' to avoid yielding a partial tag.
                                if let Some(last_lt) = buffer.rfind('<') {
                                    // Check if the content after '<' could be a start of "think>"
                                    let remaining = &buffer[last_lt..];
                                    if "<think>".starts_with(remaining) {
                                        let to_yield = buffer[..last_lt].to_string();
                                        buffer.drain(..last_lt);
                                        let to_yield = match to_yield.trim().is_empty() {
                                            true => None,
                                            false => Some(to_yield),
                                        };
                                        if let Some(text) = to_yield {
                                             yield Message {
                                                content: Content::Text { text: Some(text), thinking: None },
                                                tokens,
                                            }
                                        }
                                        break; // Wait for next chunk
                                    }
                                }

                                let to_yield = buffer.clone();
                                buffer.clear();
                                let to_yield = match to_yield.trim().is_empty() {
                                    true => None,
                                    false => Some(to_yield),
                                };
                                if let Some(text) = to_yield {
                                    yield Message {
                                        content: Content::Text { text: Some(text), thinking: None },
                                        tokens,
                                    }
                                }
                                break;
                            }
                        } else if let Some(pos) = buffer.find("</think>") {
                            let thinking = buffer[..pos].to_string();
                            buffer.drain(..pos + 8);
                            in_think_block = false;

                            let thinking = match thinking.trim().is_empty() {
                                true => None,
                                false => Some(thinking),
                            };

                            yield Message {
                                content: Content::Text { text: None, thinking },
                                tokens: None,
                            }
                        } else {
                            // No </think> tag found.
                            // We can yield everything up to the last '<' to avoid yielding a partial tag.
                            if let Some(last_lt) = buffer.rfind('<') {
                                let remaining = &buffer[last_lt..];
                                if "</think>".starts_with(remaining) {
                                    let to_yield = buffer[..last_lt].to_string();
                                    buffer.drain(..last_lt);
                                    let to_yield = match to_yield.trim().is_empty() {
                                        true => None,
                                        false => Some(to_yield),
                                    };
                                    if let Some(thinking) = to_yield {
                                         yield Message {
                                            content: Content::Text { text: None, thinking: Some(thinking) },
                                            tokens,
                                        }
                                    }
                                    break;
                                }
                            }

                            let to_yield = buffer.clone();
                            buffer.clear();
                            let to_yield = match to_yield.trim().is_empty() {
                                true => None,
                                false => Some(to_yield),
                            };
                            if let Some(thinking) = to_yield {
                                yield Message {
                                    content: Content::Text { text: None, thinking: Some(thinking) },
                                    tokens,
                                }
                            }
                            break;
                        }
                    }
                }
            }
        }
        metrics::histogram!(
            "llm_time_to_last_token_ms",
            "service" => service,
            "model" => model,
        ).record(start_time.elapsed().as_millis() as f64);
    }
    .boxed()
}

#[instrument(skip(openai_config, messages), err)]
pub async fn openai_single_tool_call<T: DeserializeOwned + JsonSchema>(
    config: CallConfig,
    openai_config: OpenAIConfig,
    temperature: Option<f32>,
    reasoning_effort: Option<ReasoningEffort>,
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
        reasoning_effort,
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

#[cfg(test)]
mod tests {
    use super::*;
    use async_openai::types::chat::{
        ChatCompletionMessageToolCall, ChatCompletionMessageToolCallChunk, CreateChatCompletionResponse,
        CreateChatCompletionStreamResponse,
    };
    use futures::stream;

    #[test]
    fn test_non_streaming_no_tools() {
        let json = r#"{
            "id": "test",
            "object": "chat.completion",
            "created": 0,
            "model": "test",
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "<think>thinking hard</think>The answer is 42"
                    }
                }
            ],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 20,
                "total_tokens": 30
            }
        }"#;
        let response: CreateChatCompletionResponse = serde_json::from_str(json).unwrap();

        let message: Message = response.try_into().unwrap();
        if let Content::Text { text, thinking } = message.content {
            assert_eq!(thinking, Some("thinking hard".to_string()));
            assert_eq!(text, Some("The answer is 42".to_string()));
        } else {
            panic!("Expected Text content");
        }
    }

    #[test]
    fn test_non_streaming_with_tools() {
        let json = r#"{
            "id": "call_1",
            "type": "function",
            "function": {
                "name": "test_tool",
                "arguments": "{\"arg\": \"<think>parsing json</think>value\"}"
            }
        }"#;
        let tool_call: ChatCompletionMessageToolCall = serde_json::from_str(json).unwrap();

        let response = ToolCallResponse::try_from(tool_call).unwrap();
        assert_eq!(response.name, "test_tool");
        assert_eq!(response.thinking, Some("parsing json".to_string()));
        assert_eq!(response.arguments["arg"], "value");
    }

    #[tokio::test]
    async fn test_streaming_no_tools() {
        let chunks = vec![
            Ok(serde_json::from_str::<CreateChatCompletionStreamResponse>(
                r#"{
                "id": "1",
                "object": "chat.completion.chunk",
                "created": 0,
                "model": "test",
                "choices": [
                    {
                        "index": 0,
                        "delta": {
                            "content": "Hello <thi"
                        }
                    }
                ]
            }"#,
            )
            .unwrap()),
            Ok(serde_json::from_str::<CreateChatCompletionStreamResponse>(
                r#"{
                "id": "2",
                "object": "chat.completion.chunk",
                "created": 0,
                "model": "test",
                "choices": [
                    {
                        "index": 0,
                        "delta": {
                            "content": "nk>thought</think>world"
                        }
                    }
                ]
            }"#,
            )
            .unwrap()),
        ];

        let stream = stream::iter(chunks);
        let mut processed = process_stream(stream, Instant::now(), "test".to_string(), "test".to_string());

        // First message should be "Hello "
        let msg1 = processed.next().await.unwrap().unwrap();
        if let Content::Text { text, thinking } = msg1.content {
            assert_eq!(text, Some("Hello ".to_string()));
            assert_eq!(thinking, None);
        } else {
            panic!("Expected Text content");
        }

        // Second message should be the thinking block
        let msg2 = processed.next().await.unwrap().unwrap();
        if let Content::Text { text, thinking } = msg2.content {
            assert_eq!(text, None);
            assert_eq!(thinking, Some("thought".to_string()));
        } else {
            panic!("Expected Text content");
        }

        // Third message should be "world"
        let msg3 = processed.next().await.unwrap().unwrap();
        if let Content::Text { text, thinking } = msg3.content {
            assert_eq!(text, Some("world".to_string()));
            assert_eq!(thinking, None);
        } else {
            panic!("Expected Text content");
        }
    }

    #[test]
    fn test_streaming_with_tools() {
        let json = r#"{
            "index": 0,
            "id": "call_1",
            "type": "function",
            "function": {
                "name": "test_tool",
                "arguments": "{\"arg\": \"<think>streaming tool thoughts</think>done\"}"
            }
        }"#;
        let tool_call_chunk: ChatCompletionMessageToolCallChunk = serde_json::from_str(json).unwrap();

        let response = ToolCallResponse::try_from(tool_call_chunk).unwrap();
        assert_eq!(response.name, "test_tool");
        assert_eq!(response.thinking, Some("streaming tool thoughts".to_string()));
        assert_eq!(response.arguments["arg"], "done");
    }
}
