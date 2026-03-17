//! Chat completions API for llama.cpp.

use crate::client::LlamaCppClient;
use crate::client::endpoints;
use crate::error::{LlamaCppError, Result};
use crate::types::{ChatMessage, ChatResponse, FinishReason, Role};

/// Client for the chat completions API.
#[derive(Debug)]
pub struct Chat<'a> {
    client: &'a LlamaCppClient,
}

impl<'a> Chat<'a> {
    /// Create a new chat client.
    pub fn new(client: &'a LlamaCppClient) -> Self {
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
    ) -> Result<impl futures::Stream<Item = Result<ChatCompletionChunk>>> {
        use eventsource_stream::Eventsource;
        use futures::StreamExt;

        request.stream = Some(true);
        let body = serde_json::to_value(&request)?;

        let url = format!("{}{}", self.client.base_url(), endpoints::CHAT_COMPLETIONS);

        tracing::debug!("Initiating streaming chat completion request");

        let response = self
            .client
            .http()
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(LlamaCppError::from)?;

        let status = response.status();
        if !status.is_success() {
            return Err(LlamaCppError::from_response(response).await);
        }

        let stream = response
            .bytes_stream()
            .eventsource()
            .map(|event| match event {
                Ok(event) => {
                    if event.data == "[DONE]" {
                        return Ok(ChatCompletionChunk::done());
                    }

                    match serde_json::from_str::<ChatCompletionChunk>(&event.data) {
                        Ok(chunk) => Ok(chunk),
                        Err(e) => Err(LlamaCppError::Stream {
                            message: format!("Failed to parse SSE event: {e}"),
                        }),
                    }
                }
                Err(e) => Err(LlamaCppError::Stream {
                    message: format!("SSE error: {e}"),
                }),
            });

        Ok(stream)
    }
}

/// A chat completion request.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChatRequest {
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
    /// Top-k sampling parameter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i32>,
    /// Minimum probability for sampling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_p: Option<f32>,
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
    /// Grammar (JSON schema or BNF grammar).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grammar: Option<String>,
    /// JSON mode - force JSON output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json_mode: Option<bool>,
    /// Cache the prompt for faster subsequent requests.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_prompt: Option<bool>,
    /// Slot ID to use (-1 for automatic assignment).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slot_id: Option<i32>,
}

impl ChatRequest {
    /// Create a new chat request builder.
    pub fn builder() -> ChatRequestBuilder {
        ChatRequestBuilder::new()
    }

    /// Create a simple chat request.
    pub fn simple(message: impl Into<String>) -> Self {
        Self::builder().message(Role::User, message).build()
    }

    /// Add a message to the conversation.
    pub fn add_message(&mut self, role: Role, content: impl Into<String>) {
        self.messages.push(ChatMessage::new(role, content));
    }
}

/// Builder for chat requests.
#[derive(Debug, Clone)]
pub struct ChatRequestBuilder {
    messages: Vec<ChatMessage>,
    max_tokens: Option<usize>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    top_k: Option<i32>,
    min_p: Option<f32>,
    stream: Option<bool>,
    stop: Option<Vec<String>>,
    presence_penalty: Option<f32>,
    frequency_penalty: Option<f32>,
    n: Option<usize>,
    seed: Option<i64>,
    grammar: Option<String>,
    json_mode: Option<bool>,
    cache_prompt: Option<bool>,
    slot_id: Option<i32>,
}

impl ChatRequestBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            max_tokens: None,
            temperature: None,
            top_p: None,
            top_k: None,
            min_p: None,
            stream: None,
            stop: None,
            presence_penalty: None,
            frequency_penalty: None,
            n: None,
            seed: None,
            grammar: None,
            json_mode: None,
            cache_prompt: None,
            slot_id: None,
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

    /// Set top-k.
    pub fn top_k(mut self, top_k: i32) -> Self {
        self.top_k = Some(top_k.max(1));
        self
    }

    /// Set min-p.
    pub fn min_p(mut self, min_p: f32) -> Self {
        self.min_p = Some(min_p.clamp(0.0, 1.0));
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

    /// Set grammar (JSON schema or BNF grammar).
    pub fn grammar(mut self, grammar: impl Into<String>) -> Self {
        self.grammar = Some(grammar.into());
        self
    }

    /// Enable JSON mode.
    pub fn json_mode(mut self, enabled: bool) -> Self {
        self.json_mode = Some(enabled);
        self
    }

    /// Enable prompt caching.
    pub fn cache_prompt(mut self, enabled: bool) -> Self {
        self.cache_prompt = Some(enabled);
        self
    }

    /// Set specific slot ID.
    pub fn slot_id(mut self, slot_id: i32) -> Self {
        self.slot_id = Some(slot_id);
        self
    }

    /// Build the request.
    pub fn build(self) -> ChatRequest {
        ChatRequest {
            messages: self.messages,
            max_tokens: self.max_tokens,
            temperature: self.temperature,
            top_p: self.top_p,
            top_k: self.top_k,
            min_p: self.min_p,
            stream: self.stream,
            stop: self.stop,
            presence_penalty: self.presence_penalty,
            frequency_penalty: self.frequency_penalty,
            n: self.n,
            seed: self.seed,
            grammar: self.grammar,
            json_mode: self.json_mode,
            cache_prompt: self.cache_prompt,
            slot_id: self.slot_id,
        }
    }
}

impl Default for ChatRequestBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// A chat completion chunk in a stream.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChatCompletionChunk {
    /// Unique identifier for the chunk.
    #[serde(rename = "id")]
    pub id: String,
    /// The object type.
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
        self.choices.is_empty()
            || self
                .choices
                .iter()
                .all(|c| c.delta.content.is_none() && c.finish_reason.is_some())
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
        self.choices
            .first()
            .and_then(|c| c.delta.content.as_deref())
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
}

impl StreamDelta {
    /// Check if this delta is empty.
    pub fn is_empty(&self) -> bool {
        self.role.is_none() && self.content.is_none()
    }
}

/// A collector that accumulates streaming chunks into a complete response.
#[derive(Debug, Default)]
pub struct StreamCollector {
    message_id: Option<String>,
    model: Option<String>,
    content: String,
    finish_reason: Option<FinishReason>,
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

            if let Some(reason) = choice.finish_reason {
                self.finish_reason = Some(reason);
            }
        }
    }

    /// Get the collected content so far.
    pub fn content(&self) -> &str {
        &self.content
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
        let request = ChatRequest::builder()
            .max_tokens(1024)
            .user("Hello")
            .temperature(0.7)
            .build();

        assert_eq!(request.max_tokens, Some(1024));
        assert_eq!(request.temperature, Some(0.7));
        assert_eq!(request.messages.len(), 1);
    }

    #[test]
    fn test_builder_messages() {
        let request = ChatRequest::builder()
            .system("You are a helpful assistant")
            .user("Hello")
            .assistant("Hi there!")
            .build();

        assert_eq!(request.messages.len(), 3);
        assert_eq!(request.messages[0].role, Role::System);
        assert_eq!(request.messages[1].role, Role::User);
        assert_eq!(request.messages[2].role, Role::Assistant);
    }

    #[test]
    fn test_json_mode() {
        let request = ChatRequest::builder()
            .user("Generate JSON")
            .json_mode(true)
            .build();

        assert_eq!(request.json_mode, Some(true));
    }

    #[test]
    fn test_cache_prompt() {
        let request = ChatRequest::builder()
            .user("Hello")
            .cache_prompt(true)
            .build();

        assert_eq!(request.cache_prompt, Some(true));
    }

    #[test]
    fn test_slot_id() {
        let request = ChatRequest::builder().user("Hello").slot_id(0).build();

        assert_eq!(request.slot_id, Some(0));
    }

    #[test]
    fn test_stream_collector() {
        let mut collector = StreamCollector::new();

        let chunk1 = ChatCompletionChunk {
            id: "chatcmpl-123".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 1234567890,
            model: "llama-3-8b".to_string(),
            choices: vec![StreamChoice {
                index: 0,
                delta: StreamDelta {
                    role: Some(Role::Assistant),
                    content: Some("Hello".to_string()),
                },
                finish_reason: None,
            }],
        };
        collector.process_chunk(&chunk1);

        let chunk2 = ChatCompletionChunk {
            id: "chatcmpl-123".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 1234567890,
            model: "llama-3-8b".to_string(),
            choices: vec![StreamChoice {
                index: 0,
                delta: StreamDelta {
                    role: None,
                    content: Some(" world!".to_string()),
                },
                finish_reason: Some(FinishReason::Stop),
            }],
        };
        collector.process_chunk(&chunk2);

        assert_eq!(collector.content(), "Hello world!");
        assert!(collector.is_complete());
    }

    #[test]
    fn test_stream_delta_empty() {
        let delta = StreamDelta::default();
        assert!(delta.is_empty());
    }
}
