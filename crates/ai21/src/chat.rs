//! Chat API for AI21's Jamba models.

use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::client::Ai21Client;
use crate::constants::endpoints;
use crate::error::{Ai21Error, Result};
use crate::types::{FinishReason, Message, ResponseFormat, Tool, ToolCall, Usage};

#[cfg(feature = "streaming")]
use crate::streaming::StreamEvent;

/// Client for the Chat API.
#[derive(Debug)]
pub struct ChatEndpoint<'a> {
    pub(crate) client: &'a Ai21Client,
}

impl<'a> ChatEndpoint<'a> {
    /// Send a chat request and get a complete response.
    pub async fn create(&self, request: ChatRequest) -> Result<ChatResponse> {
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

    /// Send a streaming chat request.
    #[cfg(feature = "streaming")]
    pub async fn stream(
        &self,
        request: ChatRequest,
    ) -> Result<impl futures::Stream<Item = Result<StreamEvent>>> {
        use eventsource_stream::Eventsource;
        use futures::StreamExt;
        use reqwest::header::AUTHORIZATION;

        let mut body = serde_json::to_value(&request)?;
        body["stream"] = serde_json::json!(true);

        let url = format!("{}{}", self.client.base_url(), endpoints::CHAT_COMPLETIONS);

        debug!("Initiating streaming chat request");

        let response = self
            .client
            .inner
            .http
            .post(&url)
            .header(
                AUTHORIZATION,
                format!("Bearer {}", self.client.inner.api_key.expose_secret()),
            )
            .json(&body)
            .send()
            .await
            .map_err(Ai21Error::from)?;

        let status = response.status();
        if !status.is_success() {
            return Err(Ai21Error::from_response(response).await);
        }

        let stream = response
            .bytes_stream()
            .eventsource()
            .map(|event| match event {
                Ok(event) => {
                    if event.data == "[DONE]" {
                        return Ok(StreamEvent::End {
                            finish_reason: Some(FinishReason::Stop),
                        });
                    }

                    match serde_json::from_str::<StreamEvent>(&event.data) {
                        Ok(stream_event) => Ok(stream_event),
                        Err(e) => Err(Ai21Error::Stream {
                            message: format!("Failed to parse SSE event: {e}"),
                        }),
                    }
                }
                Err(e) => Err(Ai21Error::Stream {
                    message: format!("SSE error: {e}"),
                }),
            });

        Ok(stream)
    }
}

/// A request to the AI21 Chat API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    /// The model to use (e.g., "jamba-1.5-large").
    pub model: String,

    /// The messages to send to the model.
    pub messages: Vec<Message>,

    /// The maximum number of tokens to generate.
    #[serde(rename = "max_tokens", skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<usize>,

    /// Whether to stream the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    /// Temperature for sampling (0.0 to 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Top-p sampling parameter (0.0 to 1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,

    /// Nucleus sampling parameter.
    #[serde(rename = "top_p_returns", skip_serializing_if = "Option::is_none")]
    pub top_p_returns: Option<usize>,

    /// Stop sequences.
    #[serde(rename = "stop", skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,

    /// Random seed for reproducibility.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,

    /// Tools available to the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,

    /// Tool choice configuration.
    #[serde(rename = "tool_choice", skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,

    /// Response format configuration.
    #[serde(rename = "response_format", skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,

    /// Number of completions to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<usize>,
}

impl ChatRequest {
    /// Create a new request builder for the given model.
    pub fn builder(model: impl Into<String>) -> ChatRequestBuilder {
        ChatRequestBuilder::new(model)
    }

    /// Create a simple request with a single user message.
    pub fn simple(model: impl Into<String>, message: impl Into<String>) -> Self {
        Self::builder(model)
            .messages(vec![Message::user(message)])
            .build()
    }
}

/// Builder for chat requests.
#[derive(Debug, Clone)]
pub struct ChatRequestBuilder {
    model: String,
    messages: Vec<Message>,
    max_tokens: Option<usize>,
    stream: Option<bool>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    top_p_returns: Option<usize>,
    stop: Option<Vec<String>>,
    seed: Option<u64>,
    tools: Option<Vec<Tool>>,
    tool_choice: Option<ToolChoice>,
    response_format: Option<ResponseFormat>,
    n: Option<usize>,
}

impl ChatRequestBuilder {
    /// Create a new builder for the given model.
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            messages: Vec::new(),
            max_tokens: None,
            stream: None,
            temperature: None,
            top_p: None,
            top_p_returns: None,
            stop: None,
            seed: None,
            tools: None,
            tool_choice: None,
            response_format: None,
            n: None,
        }
    }

    /// Set the messages.
    pub fn messages(mut self, messages: Vec<Message>) -> Self {
        self.messages = messages;
        self
    }

    /// Add a message.
    pub fn add_message(mut self, message: Message) -> Self {
        self.messages.push(message);
        self
    }

    /// Add a user message.
    pub fn user_message(mut self, content: impl Into<String>) -> Self {
        self.messages.push(Message::user(content));
        self
    }

    /// Add a system message.
    pub fn system_message(mut self, content: impl Into<String>) -> Self {
        self.messages.push(Message::system(content));
        self
    }

    /// Set the maximum tokens.
    pub fn max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Enable/disable streaming.
    pub fn stream(mut self, enabled: bool) -> Self {
        self.stream = Some(enabled);
        self
    }

    /// Set the temperature.
    pub fn temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp.clamp(0.0, 2.0));
        self
    }

    /// Set the top-p parameter.
    pub fn top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p.clamp(0.0, 1.0));
        self
    }

    /// Set the top-p returns parameter.
    pub fn top_p_returns(mut self, returns: usize) -> Self {
        self.top_p_returns = Some(returns);
        self
    }

    /// Add a stop sequence.
    pub fn stop_sequence(mut self, seq: impl Into<String>) -> Self {
        self.stop.get_or_insert_with(Vec::new).push(seq.into());
        self
    }

    /// Set the stop sequences.
    pub fn stop(mut self, seqs: Vec<String>) -> Self {
        self.stop = Some(seqs);
        self
    }

    /// Set the random seed.
    pub fn seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Add a tool.
    pub fn add_tool(mut self, tool: Tool) -> Self {
        self.tools.get_or_insert_with(Vec::new).push(tool);
        self
    }

    /// Set the tools.
    pub fn tools(mut self, tools: Vec<Tool>) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Set the tool choice.
    pub fn tool_choice(mut self, choice: ToolChoice) -> Self {
        self.tool_choice = Some(choice);
        self
    }

    /// Force the model to use a tool.
    pub fn force_tool(mut self) -> Self {
        self.tool_choice = Some(ToolChoice::Auto);
        self
    }

    /// Set the response format.
    pub fn response_format(mut self, format: ResponseFormat) -> Self {
        self.response_format = Some(format);
        self
    }

    /// Request JSON mode response.
    pub fn json_mode(mut self) -> Self {
        self.response_format = Some(ResponseFormat::json_object());
        self
    }

    /// Set the number of completions.
    pub fn n(mut self, n: usize) -> Self {
        self.n = Some(n);
        self
    }

    /// Build the request.
    pub fn build(self) -> ChatRequest {
        ChatRequest {
            model: self.model,
            messages: self.messages,
            max_tokens: self.max_tokens,
            stream: self.stream,
            temperature: self.temperature,
            top_p: self.top_p,
            top_p_returns: self.top_p_returns,
            stop: self.stop,
            seed: self.seed,
            tools: self.tools,
            tool_choice: self.tool_choice,
            response_format: self.response_format,
            n: self.n,
        }
    }
}

/// Tool choice configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolChoice {
    /// Let the model decide whether to use tools.
    Auto,
    /// Force the model to use a tool.
    Required,
    /// Do not use tools.
    None,
}

/// A response from the AI21 Chat API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    /// The ID of the response.
    pub id: String,

    /// The object type (always "chat.completion").
    pub object: String,

    /// The creation timestamp.
    pub created: u64,

    /// The model used.
    pub model: String,

    /// The list of completion choices.
    pub choices: Vec<ChatChoice>,

    /// Usage information.
    pub usage: Usage,
}

impl ChatResponse {
    /// Get the message content from the first choice.
    pub fn content(&self) -> Option<&str> {
        self.choices.first().map(|c| c.message.content.as_str())
    }

    /// Check if this response contains tool calls.
    pub fn has_tool_calls(&self) -> bool {
        self.choices
            .first()
            .and_then(|c| c.message.tool_calls.as_ref())
            .map(|t| !t.is_empty())
            .unwrap_or(false)
    }

    /// Get the tool calls from the first choice.
    pub fn get_tool_calls(&self) -> &[ToolCall] {
        self.choices
            .first()
            .and_then(|c| c.message.tool_calls.as_deref())
            .unwrap_or(&[])
    }

    /// Get the finish reason from the first choice.
    pub fn finish_reason(&self) -> Option<FinishReason> {
        self.choices.first().and_then(|c| c.finish_reason)
    }
}

/// A choice in a chat response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChoice {
    /// The index of the choice.
    pub index: usize,

    /// The message generated by the model.
    pub message: ChatMessage,

    /// The finish reason.
    #[serde(rename = "finish_reason")]
    pub finish_reason: Option<FinishReason>,
}

/// A message in a chat response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// The role of the message author.
    pub role: String,

    /// The content of the message.
    pub content: String,

    /// Tool calls in the message.
    #[serde(rename = "tool_calls", skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_basic() {
        let request = ChatRequest::builder("jamba-1.5-large")
            .messages(vec![Message::user("Hello")])
            .temperature(0.7)
            .max_tokens(1024)
            .build();

        assert_eq!(request.model, "jamba-1.5-large");
        assert_eq!(request.messages.len(), 1);
        assert_eq!(request.temperature, Some(0.7));
        assert_eq!(request.max_tokens, Some(1024));
    }

    #[test]
    fn test_builder_chain() {
        let request = ChatRequest::builder("jamba-1.5-mini")
            .system_message("You are a helpful assistant.")
            .user_message("What is Rust?")
            .temperature(0.5)
            .json_mode()
            .build();

        assert_eq!(request.messages.len(), 2);
        assert!(request.response_format.is_some());
    }

    #[test]
    fn test_response_helpers() {
        let response = ChatResponse {
            id: "chat_123".to_string(),
            object: "chat.completion".to_string(),
            created: 1234567890,
            model: "jamba-1.5-large".to_string(),
            choices: vec![ChatChoice {
                index: 0,
                message: ChatMessage {
                    role: "assistant".to_string(),
                    content: "Hello!".to_string(),
                    tool_calls: Some(vec![ToolCall {
                        id: "call_1".to_string(),
                        call_type: "function".to_string(),
                        function: crate::types::ToolCallFunction {
                            name: "test".to_string(),
                            arguments: "{}".to_string(),
                        },
                    }]),
                },
                finish_reason: Some(FinishReason::Stop),
            }],
            usage: Usage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            },
        };

        assert_eq!(response.content(), Some("Hello!"));
        assert!(response.has_tool_calls());
        assert_eq!(response.finish_reason(), Some(FinishReason::Stop));
    }
}
