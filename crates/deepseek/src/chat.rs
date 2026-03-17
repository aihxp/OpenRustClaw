//! Chat completions API for DeepSeek.

use crate::client::DeepSeekClient;
use crate::client::endpoints;
use crate::error::{DeepSeekError, Result};
use crate::types::{ChatMessage, ChatResponse, Function, Role, Tool, FinishReason};

/// Client for the chat completions API.
#[derive(Debug)]
pub struct Chat<'a> {
    client: &'a DeepSeekClient,
}

impl<'a> Chat<'a> {
    /// Create a new chat client.
    pub fn new(client: &'a DeepSeekClient) -> Self {
        Self { client }
    }

    /// Send a chat completion request.
    pub async fn complete(&self, request: ChatRequest) -> Result<ChatResponse> {
        let body = serde_json::to_value(&request)?;

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let body = body.clone();
                Box::pin(async move {
                    let response = client.post(endpoints::CHAT_COMPLETIONS, body).await?;
                    let body = client.handle_response(response).await?;
                    let chat_response: ChatResponse = serde_json::from_value(body)?;
                    Ok(chat_response)
                })
            })
            .await
    }

    /// Send a streaming chat completion request.
    #[cfg(feature = "streaming")]
    pub async fn stream(
        &self,
        mut request: ChatRequest,
    ) -> Result<ChatStream> {
        use eventsource_stream::Eventsource;
        use futures::StreamExt;

        request.stream = Some(true);
        let body = serde_json::to_value(&request)?;

        let url = format!("{}{}", self.client.base_url(), endpoints::CHAT_COMPLETIONS);

        tracing::debug!("Initiating streaming chat completion request");

        let response = reqwest::Client::new()
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.client.api_key()))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(DeepSeekError::from)?;

        let status = response.status();
        if !status.is_success() {
            return Err(DeepSeekError::from_response(response).await);
        }

        let stream = response
            .bytes_stream()
            .eventsource()
            .map(|event| {
                match event {
                    Ok(event) => {
                        if event.data == "[DONE]" {
                            return Ok(ChatCompletionChunk::done());
                        }

                        match serde_json::from_str::<ChatCompletionChunk>(&event.data) {
                            Ok(chunk) => Ok(chunk),
                            Err(e) => Err(DeepSeekError::Stream {
                                message: format!("Failed to parse SSE event: {e}"),
                            }),
                        }
                    }
                    Err(e) => Err(DeepSeekError::Stream {
                        message: format!("SSE error: {e}"),
                    }),
                }
            });

        Ok(ChatStream::new(stream))
    }
}

/// A streaming chat completion response.
#[cfg(feature = "streaming")]
pub struct ChatStream {
    inner: std::pin::Pin<Box<dyn futures::Stream<Item = Result<ChatCompletionChunk>> + Send>>,
}

#[cfg(feature = "streaming")]
impl ChatStream {
    fn new<S>(stream: S) -> Self
    where
        S: futures::Stream<Item = Result<ChatCompletionChunk>> + Send + 'static,
    {
        Self {
            inner: Box::pin(stream),
        }
    }
}

#[cfg(feature = "streaming")]
impl futures::Stream for ChatStream {
    type Item = Result<ChatCompletionChunk>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}

#[cfg(feature = "streaming")]
impl std::fmt::Debug for ChatStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChatStream").finish()
    }
}

/// A chat completion request.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChatRequest {
    /// ID of the model to use.
    pub model: String,
    /// The messages to generate chat completions for.
    pub messages: Vec<ChatMessage>,
    /// Maximum number of tokens to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<usize>,
    /// Temperature for sampling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Nucleus sampling parameter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Whether to stream back partial progress.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    /// Stop sequences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    /// Presence penalty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    /// Frequency penalty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    /// Number of completions to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<usize>,
    /// Random seed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
    /// Tools available to the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    /// Tool choice strategy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    /// Response format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
    /// Whether to use parallel tool calls.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,
}

/// Tool choice strategy.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum ToolChoice {
    /// Strategy string.
    Strategy(String),
    /// Specific tool.
    Specific {
        /// Type (always "function").
        #[serde(rename = "type")]
        tool_type: String,
        /// Function specification.
        function: ToolChoiceFunction,
    },
}

/// Specific tool choice.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolChoiceFunction {
    /// The name of the function to call.
    pub name: String,
}

impl ToolChoice {
    /// Auto tool choice.
    pub fn auto() -> Self {
        ToolChoice::Strategy("auto".to_string())
    }

    /// None tool choice.
    pub fn none() -> Self {
        ToolChoice::Strategy("none".to_string())
    }

    /// Required tool choice.
    pub fn required() -> Self {
        ToolChoice::Strategy("required".to_string())
    }

    /// Specific tool choice.
    pub fn function(name: impl Into<String>) -> Self {
        ToolChoice::Specific {
            tool_type: "function".to_string(),
            function: ToolChoiceFunction { name: name.into() },
        }
    }
}

/// Response format.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResponseFormat {
    /// The type of response format.
    #[serde(rename = "type")]
    pub format_type: String,
}

impl ResponseFormat {
    /// JSON object response format.
    pub fn json_object() -> Self {
        Self {
            format_type: "json_object".to_string(),
        }
    }

    /// Text response format.
    pub fn text() -> Self {
        Self {
            format_type: "text".to_string(),
        }
    }
}

impl ChatRequest {
    /// Create a new chat request builder.
    pub fn builder(model: impl Into<String>) -> ChatRequestBuilder {
        ChatRequestBuilder::new(model)
    }

    /// Create a simple chat request.
    pub fn simple(model: impl Into<String>, message: impl Into<String>) -> Self {
        Self::builder(model)
            .message(Role::User, message)
            .build()
    }

    /// Add a message to the conversation.
    pub fn add_message(&mut self, role: Role, content: impl Into<String>) {
        self.messages.push(ChatMessage::new(role, content));
    }

    /// Add a tool to the request.
    pub fn add_tool(&mut self, function: Function) {
        self.tools
            .get_or_insert_with(Vec::new)
            .push(Tool::function(function));
    }
}

/// Builder for chat requests.
#[derive(Debug, Clone)]
pub struct ChatRequestBuilder {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: Option<usize>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    stream: Option<bool>,
    stop: Option<Vec<String>>,
    presence_penalty: Option<f32>,
    frequency_penalty: Option<f32>,
    n: Option<usize>,
    seed: Option<i64>,
    tools: Option<Vec<Tool>>,
    tool_choice: Option<ToolChoice>,
    response_format: Option<ResponseFormat>,
    parallel_tool_calls: Option<bool>,
}

impl ChatRequestBuilder {
    /// Create a new builder.
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            messages: Vec::new(),
            max_tokens: None,
            temperature: None,
            top_p: None,
            stream: None,
            stop: None,
            presence_penalty: None,
            frequency_penalty: None,
            n: None,
            seed: None,
            tools: None,
            tool_choice: None,
            response_format: None,
            parallel_tool_calls: None,
        }
    }

    /// Add a message.
    pub fn message(mut self, role: Role, content: impl Into<String>) -> Self {
        self.messages.push(ChatMessage::new(role, content));
        self
    }

    /// Add a system message.
    pub fn system(mut self, content: impl Into<String>) -> Self {
        self.messages.push(ChatMessage::system(content));
        self
    }

    /// Add a user message.
    pub fn user(mut self, content: impl Into<String>) -> Self {
        self.messages.push(ChatMessage::user(content));
        self
    }

    /// Add an assistant message.
    pub fn assistant(mut self, content: impl Into<String>) -> Self {
        self.messages.push(ChatMessage::assistant(content));
        self
    }

    /// Set the messages.
    pub fn messages(mut self, messages: Vec<ChatMessage>) -> Self {
        self.messages = messages;
        self
    }

    /// Set max tokens.
    pub fn max_tokens(mut self, tokens: usize) -> Self {
        self.max_tokens = Some(tokens);
        self
    }

    /// Set temperature.
    pub fn temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp.clamp(0.0, 2.0));
        self
    }

    /// Set top-p.
    pub fn top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p.clamp(0.0, 1.0));
        self
    }

    /// Enable streaming.
    pub fn stream(mut self, enabled: bool) -> Self {
        self.stream = Some(enabled);
        self
    }

    /// Add a stop sequence.
    pub fn stop_sequence(mut self, seq: impl Into<String>) -> Self {
        self.stop.get_or_insert_with(Vec::new).push(seq.into());
        self
    }

    /// Set stop sequences.
    pub fn stop(mut self, stop: Vec<String>) -> Self {
        self.stop = Some(stop);
        self
    }

    /// Set presence penalty.
    pub fn presence_penalty(mut self, penalty: f32) -> Self {
        self.presence_penalty = Some(penalty.clamp(-2.0, 2.0));
        self
    }

    /// Set frequency penalty.
    pub fn frequency_penalty(mut self, penalty: f32) -> Self {
        self.frequency_penalty = Some(penalty.clamp(-2.0, 2.0));
        self
    }

    /// Set number of completions.
    pub fn n(mut self, n: usize) -> Self {
        self.n = Some(n);
        self
    }

    /// Set random seed.
    pub fn seed(mut self, seed: i64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Add a tool.
    pub fn tool(mut self, function: Function) -> Self {
        self.tools
            .get_or_insert_with(Vec::new)
            .push(Tool::function(function));
        self
    }

    /// Set tools.
    pub fn tools(mut self, tools: Vec<Tool>) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Set tool choice.
    pub fn tool_choice(mut self, choice: ToolChoice) -> Self {
        self.tool_choice = Some(choice);
        self
    }

    /// Set response format to JSON.
    /// Note: When using JSON mode, you must also instruct the model to produce JSON
    /// via a system or user message.
    pub fn json_mode(mut self) -> Self {
        self.response_format = Some(ResponseFormat::json_object());
        self
    }

    /// Set response format to text (default).
    pub fn text_mode(mut self) -> Self {
        self.response_format = Some(ResponseFormat::text());
        self
    }

    /// Enable parallel tool calls.
    pub fn parallel_tool_calls(mut self, enabled: bool) -> Self {
        self.parallel_tool_calls = Some(enabled);
        self
    }

    /// Build the request.
    pub fn build(self) -> ChatRequest {
        ChatRequest {
            model: self.model,
            messages: self.messages,
            max_tokens: self.max_tokens,
            temperature: self.temperature,
            top_p: self.top_p,
            stream: self.stream,
            stop: self.stop,
            presence_penalty: self.presence_penalty,
            frequency_penalty: self.frequency_penalty,
            n: self.n,
            seed: self.seed,
            tools: self.tools,
            tool_choice: self.tool_choice,
            response_format: self.response_format,
            parallel_tool_calls: self.parallel_tool_calls,
        }
    }
}

/// A chat completion chunk in a stream.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChatCompletionChunk {
    /// Unique identifier for the chunk.
    pub id: String,
    /// The object type (always "chat.completion.chunk").
    pub object: String,
    /// The Unix timestamp.
    pub created: i64,
    /// The model used.
    pub model: String,
    /// The list of choices.
    pub choices: Vec<StreamChoice>,
}

impl ChatCompletionChunk {
    /// Check if this is the final chunk.
    pub fn is_done(&self) -> bool {
        self.choices.is_empty() || self.choices.iter().all(|c| c.delta.is_empty() && c.finish_reason.is_some())
    }

    /// Create a done chunk.
    pub fn done() -> Self {
        Self {
            id: String::new(),
            object: "chat.completion.chunk".to_string(),
            created: 0,
            model: String::new(),
            choices: vec![],
        }
    }

    /// Get the content delta from the first choice.
    pub fn content(&self) -> Option<&str> {
        self.choices.first().and_then(|c| c.delta.content.as_deref())
    }

    /// Get the reasoning content delta from the first choice (DeepSeek-R1).
    pub fn reasoning_content(&self) -> Option<&str> {
        self.choices.first().and_then(|c| c.delta.reasoning_content.as_deref())
    }

    /// Check if this chunk has reasoning content.
    pub fn has_reasoning(&self) -> bool {
        self.reasoning_content().is_some()
    }

    /// Get tool call deltas from the first choice.
    pub fn tool_calls(&self) -> Option<&Vec<ToolCallDelta>> {
        self.choices.first().and_then(|c| c.delta.tool_calls.as_ref())
    }

    /// Check if the first choice has finished.
    pub fn finish_reason(&self) -> Option<FinishReason> {
        self.choices.first().and_then(|c| c.finish_reason)
    }
}

/// A choice in a streaming chunk.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StreamChoice {
    /// The index of this choice.
    pub index: usize,
    /// The delta.
    pub delta: StreamDelta,
    /// The reason the completion finished.
    pub finish_reason: Option<FinishReason>,
    /// Logprobs (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<serde_json::Value>,
}

/// A delta in a streaming response.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct StreamDelta {
    /// The role (only present in the first chunk).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<Role>,
    /// The content delta.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// The reasoning content delta (DeepSeek-R1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
    /// Tool call deltas.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCallDelta>>,
}

impl StreamDelta {
    /// Check if this delta is empty.
    pub fn is_empty(&self) -> bool {
        self.role.is_none() 
            && self.content.is_none() 
            && self.reasoning_content.is_none() 
            && self.tool_calls.is_none()
    }
}

/// A tool call delta.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolCallDelta {
    /// The index of this tool call.
    pub index: usize,
    /// The ID of the tool call (present in first chunk).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// The type of tool call (present in first chunk).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub call_type: Option<String>,
    /// The function delta.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<FunctionDelta>,
}

/// A function delta.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FunctionDelta {
    /// The name (present in first chunk).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The arguments delta.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
}

/// A collector that accumulates streaming chunks into a complete response.
#[derive(Debug, Default)]
pub struct StreamCollector {
    message_id: Option<String>,
    model: Option<String>,
    content: String,
    reasoning_content: String,
    tool_calls: Vec<PartialToolCall>,
    finish_reason: Option<FinishReason>,
}

#[derive(Debug)]
struct PartialToolCall {
    id: String,
    name: String,
    arguments: String,
}

impl StreamCollector {
    /// Create a new stream collector.
    pub fn new() -> Self {
        Self::default()
    }

    /// Process a chunk.
    pub fn process_chunk(&mut self, chunk: &ChatCompletionChunk) {
        if !chunk.id.is_empty() {
            self.message_id = Some(chunk.id.clone());
        }
        if !chunk.model.is_empty() {
            self.model = Some(chunk.model.clone());
        }

        if let Some(choice) = chunk.choices.first() {
            if let Some(content) = &choice.delta.content {
                self.content.push_str(content);
            }

            if let Some(reasoning_content) = &choice.delta.reasoning_content {
                self.reasoning_content.push_str(reasoning_content);
            }

            if let Some(tool_call_deltas) = &choice.delta.tool_calls {
                for delta in tool_call_deltas {
                    // Ensure we have enough slots
                    while self.tool_calls.len() <= delta.index {
                        self.tool_calls.push(PartialToolCall {
                            id: String::new(),
                            name: String::new(),
                            arguments: String::new(),
                        });
                    }

                    let tool_call = &mut self.tool_calls[delta.index];

                    if let Some(id) = &delta.id {
                        tool_call.id = id.clone();
                    }
                    if let Some(function) = &delta.function {
                        if let Some(name) = &function.name {
                            tool_call.name = name.clone();
                        }
                        if let Some(args) = &function.arguments {
                            tool_call.arguments.push_str(args);
                        }
                    }
                }
            }

            if let Some(reason) = choice.finish_reason {
                self.finish_reason = Some(reason);
            }
        }
    }

    /// Get the collected content so far.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Get the collected reasoning content so far (DeepSeek-R1).
    pub fn reasoning_content(&self) -> &str {
        &self.reasoning_content
    }

    /// Check if there is any reasoning content.
    pub fn has_reasoning(&self) -> bool {
        !self.reasoning_content.is_empty()
    }

    /// Get the collected tool calls.
    pub fn tool_calls(&self) -> Vec<crate::types::ToolCall> {
        self.tool_calls
            .iter()
            .map(|tc| crate::types::ToolCall {
                id: tc.id.clone(),
                call_type: "function".to_string(),
                function: crate::types::FunctionCall {
                    name: tc.name.clone(),
                    arguments: tc.arguments.clone(),
                },
            })
            .collect()
    }

    /// Check if the stream has finished.
    pub fn is_complete(&self) -> bool {
        self.finish_reason.is_some()
    }

    /// Get the finish reason.
    pub fn finish_reason(&self) -> Option<FinishReason> {
        self.finish_reason
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_basic() {
        let request = ChatRequest::builder("deepseek-chat")
            .max_tokens(1024)
            .user("Hello")
            .temperature(0.7)
            .build();

        assert_eq!(request.model, "deepseek-chat");
        assert_eq!(request.max_tokens, Some(1024));
        assert_eq!(request.temperature, Some(0.7));
    }

    #[test]
    fn test_tool_choice() {
        let _auto = ToolChoice::auto();
        let func = ToolChoice::function("get_weather");
        let none = ToolChoice::none();

        match func {
            ToolChoice::Specific { tool_type, function } => {
                assert_eq!(tool_type, "function");
                assert_eq!(function.name, "get_weather");
            }
            _ => panic!("Expected Specific tool choice"),
        }

        assert!(matches!(none, ToolChoice::Strategy(s) if s == "none"));
    }

    #[test]
    fn test_json_mode() {
        let request = ChatRequest::builder("deepseek-chat")
            .user("Generate JSON")
            .json_mode()
            .build();

        assert!(request.response_format.is_some());
    }

    #[test]
    fn test_stream_collector() {
        let mut collector = StreamCollector::new();

        let chunk1 = ChatCompletionChunk {
            id: "chatcmpl-123".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 1234567890,
            model: "deepseek-chat".to_string(),
            choices: vec![StreamChoice {
                index: 0,
                delta: StreamDelta {
                    role: Some(Role::Assistant),
                    content: Some("Hello".to_string()),
                    reasoning_content: None,
                    tool_calls: None,
                },
                finish_reason: None,
                logprobs: None,
            }],
        };
        collector.process_chunk(&chunk1);

        let chunk2 = ChatCompletionChunk {
            id: "chatcmpl-123".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 1234567890,
            model: "deepseek-chat".to_string(),
            choices: vec![StreamChoice {
                index: 0,
                delta: StreamDelta {
                    role: None,
                    content: Some(" world!".to_string()),
                    reasoning_content: None,
                    tool_calls: None,
                },
                finish_reason: Some(FinishReason::Stop),
                logprobs: None,
            }],
        };
        collector.process_chunk(&chunk2);

        assert_eq!(collector.content(), "Hello world!");
        assert!(collector.is_complete());
    }

    #[test]
    fn test_stream_collector_with_reasoning() {
        let mut collector = StreamCollector::new();

        let chunk1 = ChatCompletionChunk {
            id: "chatcmpl-123".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 1234567890,
            model: "deepseek-reasoner".to_string(),
            choices: vec![StreamChoice {
                index: 0,
                delta: StreamDelta {
                    role: Some(Role::Assistant),
                    content: None,
                    reasoning_content: Some("Let me think".to_string()),
                    tool_calls: None,
                },
                finish_reason: None,
                logprobs: None,
            }],
        };
        collector.process_chunk(&chunk1);

        let chunk2 = ChatCompletionChunk {
            id: "chatcmpl-123".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 1234567890,
            model: "deepseek-reasoner".to_string(),
            choices: vec![StreamChoice {
                index: 0,
                delta: StreamDelta {
                    role: None,
                    content: Some("The answer is 42".to_string()),
                    reasoning_content: Some("... step by step".to_string()),
                    tool_calls: None,
                },
                finish_reason: Some(FinishReason::Stop),
                logprobs: None,
            }],
        };
        collector.process_chunk(&chunk2);

        assert_eq!(collector.content(), "The answer is 42");
        assert_eq!(collector.reasoning_content(), "Let me think... step by step");
        assert!(collector.has_reasoning());
        assert!(collector.is_complete());
    }

    #[test]
    fn test_stream_delta_empty() {
        let delta = StreamDelta::default();
        assert!(delta.is_empty());
    }

    #[test]
    fn test_seed() {
        let request = ChatRequest::builder("deepseek-chat")
            .user("Hello")
            .seed(42)
            .build();

        assert_eq!(request.seed, Some(42));
    }

    #[test]
    fn test_parallel_tool_calls() {
        let request = ChatRequest::builder("deepseek-chat")
            .user("Hello")
            .parallel_tool_calls(true)
            .build();

        assert_eq!(request.parallel_tool_calls, Some(true));
    }
}
