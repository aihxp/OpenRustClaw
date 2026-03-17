//! Chat API for Cohere's Command R and Command R+ models.

use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::client::CohereClient;
use crate::constants::endpoints;
use crate::error::{CohereError, Result};
use crate::types::{ApiMeta, Connector, Document, FinishReason, Message, MessageRole};

#[cfg(feature = "streaming")]
use crate::streaming::StreamEvent;

/// Client for the Chat API.
#[derive(Debug)]
pub struct ChatEndpoint<'a> {
    pub(crate) client: &'a CohereClient,
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
                    let response = client.post(endpoints::CHAT, body).await?;
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

        let url = format!("{}{}", self.client.base_url(), endpoints::CHAT);

        debug!("Initiating streaming chat request");

        // For streaming, we use the inner HTTP client directly
        let response = self
            .client
            .inner
            .http
            .post(&url)
            .header(AUTHORIZATION, format!("Bearer {}", self.client.inner.api_key))
            .json(&body)
            .send()
            .await
            .map_err(CohereError::from)?;

        let status = response.status();
        if !status.is_success() {
            return Err(CohereError::from_response(response).await);
        }

        let stream = response
            .bytes_stream()
            .eventsource()
            .map(|event| {
                match event {
                    Ok(event) => {
                        if event.data == "[DONE]" {
                            return Ok(StreamEvent::StreamEnd {
                                finish_reason: FinishReason::Complete,
                                generation_id: String::new(),
                                response: None,
                                meta: None,
                            });
                        }

                        match serde_json::from_str::<StreamEvent>(&event.data) {
                            Ok(stream_event) => Ok(stream_event),
                            Err(e) => Err(CohereError::Stream {
                                message: format!("Failed to parse SSE event: {e}"),
                            }),
                        }
                    }
                    Err(e) => Err(CohereError::Stream {
                        message: format!("SSE error: {e}"),
                    }),
                }
            });

        Ok(stream)
    }

    /// Send a chat request with tool results.
    pub async fn with_tool_results(
        &self,
        request: ChatRequest,
        tool_results: Vec<ToolResult>,
    ) -> Result<ChatResponse> {
        let mut body = serde_json::to_value(&request)?;
        body["tool_results"] = serde_json::to_value(&tool_results)?;

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let body = body.clone();
                Box::pin(async move {
                    let response = client.post(endpoints::CHAT, body).await?;
                    let body = client.handle_response(response).await?;
                    let chat_response: ChatResponse = serde_json::from_value(body)?;
                    Ok(chat_response)
                })
            })
            .await
    }
}

/// A request to the Cohere Chat API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    /// The model to use (e.g., "command-r").
    pub model: String,

    /// The message to send to the model.
    pub message: String,

    /// Whether to stream the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    /// A list of previous messages between the user and model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chat_history: Option<Vec<Message>>,

    /// A preamble to guide the model's behavior.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preamble: Option<String>,

    /// Documents for RAG (Retrieval-Augmented Generation).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documents: Option<Vec<Document>>,

    /// Connectors to use for search.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connectors: Option<Vec<Connector>>,

    /// Temperature for sampling (0.0 to 1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Top-p sampling parameter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub p: Option<f32>,

    /// Top-k sampling parameter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub k: Option<usize>,

    /// Maximum number of tokens to generate.
    #[serde(rename = "max_tokens", skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<usize>,

    /// Stop sequences.
    #[serde(rename = "stop_sequences", skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,

    /// Random seed for reproducibility.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,

    /// Frequency penalty (0.0 to 1.0).
    #[serde(rename = "frequency_penalty", skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,

    /// Presence penalty (0.0 to 1.0).
    #[serde(rename = "presence_penalty", skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,

    /// Tools available to the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,

    /// Force the model to use a tool.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force_single_step: Option<bool>,

    /// Response format configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
}

impl ChatRequest {
    /// Create a new request builder for the given model.
    pub fn builder(model: impl Into<String>) -> ChatRequestBuilder {
        ChatRequestBuilder::new(model)
    }

    /// Create a simple request with a single message.
    pub fn simple(model: impl Into<String>, message: impl Into<String>) -> Self {
        Self::builder(model).message(message).build()
    }

    /// Enable streaming for this request.
    pub fn with_streaming(mut self, enabled: bool) -> Self {
        self.stream = Some(enabled);
        self
    }
}

/// Builder for chat requests.
#[derive(Debug, Clone)]
pub struct ChatRequestBuilder {
    model: String,
    message: String,
    stream: Option<bool>,
    chat_history: Option<Vec<Message>>,
    preamble: Option<String>,
    documents: Option<Vec<Document>>,
    connectors: Option<Vec<Connector>>,
    temperature: Option<f32>,
    p: Option<f32>,
    k: Option<usize>,
    max_tokens: Option<usize>,
    stop_sequences: Option<Vec<String>>,
    seed: Option<u64>,
    frequency_penalty: Option<f32>,
    presence_penalty: Option<f32>,
    tools: Option<Vec<Tool>>,
    force_single_step: Option<bool>,
    response_format: Option<ResponseFormat>,
}

impl ChatRequestBuilder {
    /// Create a new builder for the given model.
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            message: String::new(),
            stream: None,
            chat_history: None,
            preamble: None,
            documents: None,
            connectors: None,
            temperature: None,
            p: None,
            k: None,
            max_tokens: None,
            stop_sequences: None,
            seed: None,
            frequency_penalty: None,
            presence_penalty: None,
            tools: None,
            force_single_step: None,
            response_format: None,
        }
    }

    /// Set the message.
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = message.into();
        self
    }

    /// Enable/disable streaming.
    pub fn stream(mut self, enabled: bool) -> Self {
        self.stream = Some(enabled);
        self
    }

    /// Add a message to the chat history.
    pub fn add_message(mut self, role: MessageRole, content: impl Into<String>) -> Self {
        self.chat_history
            .get_or_insert_with(Vec::new)
            .push(Message::new(role, content));
        self
    }

    /// Set the chat history.
    pub fn chat_history(mut self, history: Vec<Message>) -> Self {
        self.chat_history = Some(history);
        self
    }

    /// Set the preamble.
    pub fn preamble(mut self, preamble: impl Into<String>) -> Self {
        self.preamble = Some(preamble.into());
        self
    }

    /// Add a document for RAG.
    pub fn add_document(mut self, document: Document) -> Self {
        self.documents.get_or_insert_with(Vec::new).push(document);
        self
    }

    /// Set the documents for RAG.
    pub fn documents(mut self, documents: Vec<Document>) -> Self {
        self.documents = Some(documents);
        self
    }

    /// Add a connector.
    pub fn add_connector(mut self, connector: Connector) -> Self {
        self.connectors.get_or_insert_with(Vec::new).push(connector);
        self
    }

    /// Set the connectors.
    pub fn connectors(mut self, connectors: Vec<Connector>) -> Self {
        self.connectors = Some(connectors);
        self
    }

    /// Set the temperature.
    pub fn temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp.clamp(0.0, 1.0));
        self
    }

    /// Set the top-p parameter.
    pub fn p(mut self, p: f32) -> Self {
        self.p = Some(p.clamp(0.0, 1.0));
        self
    }

    /// Set the top-k parameter.
    pub fn k(mut self, k: usize) -> Self {
        self.k = Some(k);
        self
    }

    /// Set the maximum tokens.
    pub fn max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Add a stop sequence.
    pub fn stop_sequence(mut self, seq: impl Into<String>) -> Self {
        self.stop_sequences
            .get_or_insert_with(Vec::new)
            .push(seq.into());
        self
    }

    /// Set the stop sequences.
    pub fn stop_sequences(mut self, seqs: Vec<String>) -> Self {
        self.stop_sequences = Some(seqs);
        self
    }

    /// Set the random seed.
    pub fn seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Set the frequency penalty.
    pub fn frequency_penalty(mut self, penalty: f32) -> Self {
        self.frequency_penalty = Some(penalty.clamp(0.0, 1.0));
        self
    }

    /// Set the presence penalty.
    pub fn presence_penalty(mut self, penalty: f32) -> Self {
        self.presence_penalty = Some(penalty.clamp(0.0, 1.0));
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

    /// Force single step (tool use).
    pub fn force_single_step(mut self, value: bool) -> Self {
        self.force_single_step = Some(value);
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

    /// Build the request.
    pub fn build(self) -> ChatRequest {
        ChatRequest {
            model: self.model,
            message: self.message,
            stream: self.stream,
            chat_history: self.chat_history,
            preamble: self.preamble,
            documents: self.documents,
            connectors: self.connectors,
            temperature: self.temperature,
            p: self.p,
            k: self.k,
            max_tokens: self.max_tokens,
            stop_sequences: self.stop_sequences,
            seed: self.seed,
            frequency_penalty: self.frequency_penalty,
            presence_penalty: self.presence_penalty,
            tools: self.tools,
            force_single_step: self.force_single_step,
            response_format: self.response_format,
        }
    }
}

/// A response from the Cohere Chat API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    /// The generated response text.
    pub text: String,

    /// Generation ID.
    #[serde(rename = "generation_id")]
    pub generation_id: String,

    /// The role of the responder (always "assistant").
    pub role: MessageRole,

    /// The finish reason.
    #[serde(rename = "finish_reason")]
    pub finish_reason: FinishReason,

    /// The tool calls planned by the model.
    #[serde(rename = "tool_calls", skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,

    /// Documents referenced in the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documents: Option<Vec<Document>>,

    /// Citations for the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citations: Option<Vec<Citation>>,

    /// Search queries used.
    #[serde(rename = "search_queries", skip_serializing_if = "Option::is_none")]
    pub search_queries: Option<Vec<SearchQuery>>,

    /// Search results.
    #[serde(rename = "search_results", skip_serializing_if = "Option::is_none")]
    pub search_results: Option<Vec<SearchResult>>,

    /// API metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<ApiMeta>,
}

impl ChatResponse {
    /// Get the response text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Check if this response contains tool calls.
    pub fn has_tool_calls(&self) -> bool {
        self.tool_calls
            .as_ref()
            .map(|t| !t.is_empty())
            .unwrap_or(false)
    }

    /// Get the tool calls.
    pub fn get_tool_calls(&self) -> &[ToolCall] {
        self.tool_calls.as_deref().unwrap_or(&[])
    }
}

/// Response format configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseFormat {
    /// The type of response format.
    #[serde(rename = "type")]
    pub format_type: String,
}

impl ResponseFormat {
    /// Create a JSON object response format.
    pub fn json_object() -> Self {
        Self {
            format_type: "json_object".to_string(),
        }
    }

    /// Create a text response format (default).
    pub fn text() -> Self {
        Self {
            format_type: "text".to_string(),
        }
    }
}

/// A tool definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// The name of the tool.
    pub name: String,
    /// A description of what the tool does.
    pub description: String,
    /// The JSON schema for the tool's parameters.
    pub parameter_definitions: serde_json::Map<String, serde_json::Value>,
}

impl Tool {
    /// Create a new tool definition.
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        parameters: serde_json::Map<String, serde_json::Value>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameter_definitions: parameters,
        }
    }
}

/// A tool call planned by the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// The name of the tool to call.
    pub name: String,
    /// The parameters to pass to the tool.
    pub parameters: serde_json::Value,
}

/// A tool result to send back to the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// The call ID (matches the tool call).
    pub call: ToolCall,
    /// The output of the tool.
    pub outputs: Vec<serde_json::Value>,
}

impl ToolResult {
    /// Create a new tool result.
    pub fn new(call: ToolCall, outputs: Vec<serde_json::Value>) -> Self {
        Self { call, outputs }
    }
}

/// A citation in the response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    /// The start index in the response text.
    pub start: usize,
    /// The end index in the response text.
    pub end: usize,
    /// The text being cited.
    pub text: String,
    /// The document IDs referenced.
    #[serde(rename = "document_ids")]
    pub document_ids: Vec<String>,
}

/// A search query generated by the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    /// The search query text.
    pub text: String,
    /// The connector used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connector: Option<String>,
}

/// A search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// The connector used.
    pub connector: String,
    /// The search results.
    pub documents: Vec<Document>,
}

/// Streaming response type alias.
#[cfg(feature = "streaming")]
pub type ChatStreamResponse = crate::streaming::StreamEvent;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_basic() {
        let request = ChatRequest::builder("command-r")
            .message("Hello")
            .temperature(0.7)
            .max_tokens(1024)
            .build();

        assert_eq!(request.model, "command-r");
        assert_eq!(request.message, "Hello");
        assert_eq!(request.temperature, Some(0.7));
        assert_eq!(request.max_tokens, Some(1024));
    }

    #[test]
    fn test_builder_chain() {
        let request = ChatRequest::builder("command-r-plus")
            .message("What is Rust?")
            .preamble("You are a helpful assistant.")
            .add_message(MessageRole::User, "Hello")
            .add_message(MessageRole::Assistant, "Hi!")
            .temperature(0.5)
            .json_mode()
            .build();

        assert_eq!(request.chat_history.as_ref().map(|h| h.len()), Some(2));
        assert!(request.preamble.is_some());
        assert!(request.response_format.is_some());
    }

    #[test]
    fn test_tool_creation() {
        let mut params = serde_json::Map::new();
        params.insert(
            "location".to_string(),
            serde_json::json!({"type": "string", "description": "The city"}),
        );

        let tool = Tool::new("get_weather", "Get the weather for a location", params);

        assert_eq!(tool.name, "get_weather");
        assert_eq!(tool.description, "Get the weather for a location");
    }

    #[test]
    fn test_response_helpers() {
        let response = ChatResponse {
            text: "Hello!".to_string(),
            generation_id: "gen_123".to_string(),
            role: MessageRole::Assistant,
            finish_reason: FinishReason::Complete,
            tool_calls: Some(vec![ToolCall {
                name: "test".to_string(),
                parameters: serde_json::json!({}),
            }]),
            documents: None,
            citations: None,
            search_queries: None,
            search_results: None,
            meta: None,
        };

        assert_eq!(response.text(), "Hello!");
        assert!(response.has_tool_calls());
    }
}
