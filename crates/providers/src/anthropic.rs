//! Anthropic Messages API provider.
//!
//! Implements the [`LlmProvider`] trait by calling the Anthropic Messages API
//! (`POST /v1/messages`) via `reqwest`.

use std::pin::Pin;

use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures::{Stream, StreamExt};
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue};
use secrecy::{ExposeSecret, SecretString};
use serde_json::Value;
use tracing::{debug, warn};

use openrustclaw_core::error::{Error, ProviderError, Result};
use openrustclaw_core::traits::LlmProvider;
use openrustclaw_core::types::{
    CompletionRequest, CompletionResponse, FinishReason, Message, Role, StreamChunk, TokenUsage,
    ToolFormat,
};

use crate::tool_formats::{parse_anthropic_tool_calls, translate_tool_definition};

/// Default base URL for the Anthropic Messages API.
const DEFAULT_BASE_URL: &str = "https://api.anthropic.com";

/// Default API version header value.
const DEFAULT_API_VERSION: &str = "2023-06-01";

/// LLM provider implementation for Anthropic's Messages API.
pub struct AnthropicProvider {
    client: reqwest::Client,
    api_key: SecretString,
    model: String,
    api_version: String,
    base_url: String,
}

impl AnthropicProvider {
    /// Create a new Anthropic provider with the given API key and model.
    ///
    /// Uses default values for `api_version` (`2023-06-01`) and `base_url`
    /// (`https://api.anthropic.com`).
    pub fn new(api_key: impl Into<SecretString>, model: String) -> Self {
        Self {
            client: reqwest::Client::builder()
                .connect_timeout(std::time::Duration::from_secs(10))
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("Failed to build HTTP client"),
            api_key: api_key.into(),
            model,
            api_version: DEFAULT_API_VERSION.to_string(),
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }

    /// Create a new Anthropic provider with custom base URL and API version.
    pub fn with_config(
        api_key: impl Into<SecretString>,
        model: String,
        base_url: String,
        api_version: String,
    ) -> Self {
        Self {
            client: reqwest::Client::builder()
                .connect_timeout(std::time::Duration::from_secs(10))
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("Failed to build HTTP client"),
            api_key: api_key.into(),
            model,
            api_version,
            base_url,
        }
    }

    /// Build the request body for the Anthropic Messages API from a
    /// [`CompletionRequest`].
    fn build_request_body(&self, request: &CompletionRequest) -> Value {
        let model = request.model.as_deref().unwrap_or(&self.model);

        // Separate system messages from conversation messages.
        // Anthropic uses a top-level `system` field rather than a system role message.
        let system_prompt = request.system_prompt.clone().or_else(|| {
            request
                .messages
                .iter()
                .find(|m| m.role == Role::System)
                .map(|m| m.content.clone())
        });

        // Build the messages array (excluding system messages).
        let messages: Vec<Value> = request
            .messages
            .iter()
            .filter(|m| m.role != Role::System)
            .map(|m| self.message_to_anthropic(m))
            .collect();

        let max_tokens = request.max_tokens.unwrap_or(4096);

        let mut body = serde_json::json!({
            "model": model,
            "max_tokens": max_tokens,
            "messages": messages,
        });

        if let Some(system) = system_prompt {
            body["system"] = Value::String(system);
        }

        if let Some(temp) = request.temperature {
            body["temperature"] = serde_json::json!(temp);
        }

        if let Some(tools) = &request.tools
            && !tools.is_empty()
        {
            let tool_defs: Vec<Value> = tools
                .iter()
                .map(|t| translate_tool_definition(t, ToolFormat::Anthropic))
                .collect();
            body["tools"] = Value::Array(tool_defs);
        }

        body
    }

    /// Convert a unified [`Message`] to the Anthropic message format.
    fn message_to_anthropic(&self, msg: &Message) -> Value {
        let role = match msg.role {
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::Tool => "user", // Anthropic wraps tool results in user messages
            Role::System => "user", // should not reach here (filtered above)
        };

        // Handle tool result messages: wrap in a tool_result content block.
        if msg.role == Role::Tool
            && let Some(ref tool_call_id) = msg.tool_call_id
        {
            return serde_json::json!({
                "role": "user",
                "content": [{
                    "type": "tool_result",
                    "tool_use_id": tool_call_id,
                    "content": msg.content,
                }]
            });
        }

        // Handle assistant messages with tool calls: include content blocks.
        if msg.role == Role::Assistant
            && let Some(ref tool_calls) = msg.tool_calls
            && !tool_calls.is_empty()
        {
            let mut content_blocks: Vec<Value> = Vec::new();

            // Add text content if present.
            if !msg.content.is_empty() {
                content_blocks.push(serde_json::json!({
                    "type": "text",
                    "text": msg.content,
                }));
            }

            // Add tool_use blocks.
            for tc in tool_calls {
                content_blocks.push(serde_json::json!({
                    "type": "tool_use",
                    "id": tc.id,
                    "name": tc.name,
                    "input": tc.arguments,
                }));
            }

            return serde_json::json!({
                "role": "assistant",
                "content": content_blocks,
            });
        }

        serde_json::json!({
            "role": role,
            "content": msg.content,
        })
    }

    /// Parse the Anthropic API response JSON into a [`CompletionResponse`].
    fn parse_response(&self, body: Value) -> Result<CompletionResponse> {
        let id = body
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let model = body
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or(&self.model)
            .to_string();

        let stop_reason = body
            .get("stop_reason")
            .and_then(|v| v.as_str())
            .unwrap_or("end_turn");

        let finish_reason = match stop_reason {
            "end_turn" | "stop" => FinishReason::Stop,
            "tool_use" => FinishReason::ToolUse,
            "max_tokens" => FinishReason::MaxTokens,
            _ => FinishReason::Stop,
        };

        // Extract content blocks.
        let content_blocks = body
            .get("content")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        // Extract text content from text blocks.
        let text_content: String = content_blocks
            .iter()
            .filter(|block| block.get("type").and_then(|t| t.as_str()) == Some("text"))
            .filter_map(|block| block.get("text").and_then(|t| t.as_str()))
            .collect::<Vec<&str>>()
            .join("");

        // Extract tool calls from tool_use blocks.
        let tool_calls = parse_anthropic_tool_calls(&content_blocks);

        // Build the message.
        let mut message = Message::assistant(text_content);
        if !tool_calls.is_empty() {
            message.tool_calls = Some(tool_calls);
        }

        // Parse usage.
        let usage = if let Some(usage_obj) = body.get("usage") {
            TokenUsage {
                prompt_tokens: usage_obj
                    .get("input_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as usize,
                completion_tokens: usage_obj
                    .get("output_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as usize,
                total_tokens: {
                    let input = usage_obj
                        .get("input_tokens")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as usize;
                    let output = usage_obj
                        .get("output_tokens")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as usize;
                    input + output
                },
                cost_usd: None,
            }
        } else {
            TokenUsage::default()
        };

        Ok(CompletionResponse {
            id,
            message,
            model,
            usage,
            provider: "anthropic".to_string(),
            finish_reason,
        })
    }

    /// Build default headers for Anthropic API requests.
    fn default_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            "x-api-key",
            HeaderValue::from_str(self.api_key.expose_secret())
                .unwrap_or_else(|_| HeaderValue::from_static("")),
        );
        headers.insert(
            "anthropic-version",
            HeaderValue::from_str(&self.api_version)
                .unwrap_or_else(|_| HeaderValue::from_static(DEFAULT_API_VERSION)),
        );
        headers
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let url = format!("{}/v1/messages", self.base_url);
        let body = self.build_request_body(&request);

        debug!(provider = "anthropic", model = %self.model, "Sending completion request");

        let response = self
            .client
            .post(&url)
            .headers(self.default_headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                Error::Provider(ProviderError::Request(format!(
                    "Anthropic request failed: {e}"
                )))
            })?;

        let status = response.status();

        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok());
            return Err(Error::Provider(ProviderError::RateLimited {
                provider: "anthropic".to_string(),
                retry_after_secs: retry_after,
            }));
        }

        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            let error_body = response.text().await.unwrap_or_default();
            return Err(Error::Provider(ProviderError::AuthFailed {
                provider: "anthropic".to_string(),
                message: error_body,
            }));
        }

        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::Provider(ProviderError::ModelNotFound {
                provider: "anthropic".to_string(),
                model: self.model.clone(),
            }));
        }

        if status.is_server_error() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(Error::Provider(ProviderError::Unavailable {
                provider: "anthropic".to_string(),
                message: format!("Server error {status}: {error_body}"),
            }));
        }

        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            warn!(
                provider = "anthropic",
                status = %status,
                body = %error_body,
                "Unexpected error response"
            );
            return Err(Error::Provider(ProviderError::Request(format!(
                "Anthropic API error {status}: {error_body}"
            ))));
        }

        let response_body: Value = response.json().await.map_err(|e| {
            Error::Provider(ProviderError::Parse(format!(
                "Failed to parse Anthropic response: {e}"
            )))
        })?;

        self.parse_response(response_body)
    }

    async fn stream(
        &self,
        request: CompletionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>> {
        let url = format!("{}/v1/messages", self.base_url);
        let mut body = self.build_request_body(&request);
        body["stream"] = serde_json::json!(true);

        debug!(provider = "anthropic", model = %self.model, "Sending streaming completion request");

        let response = self
            .client
            .post(&url)
            .headers(self.default_headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                Error::Provider(ProviderError::Request(format!(
                    "Anthropic streaming request failed: {e}"
                )))
            })?;

        let status = response.status();

        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok());
            return Err(Error::Provider(ProviderError::RateLimited {
                provider: "anthropic".to_string(),
                retry_after_secs: retry_after,
            }));
        }

        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            let error_body = response.text().await.unwrap_or_default();
            return Err(Error::Provider(ProviderError::AuthFailed {
                provider: "anthropic".to_string(),
                message: error_body,
            }));
        }

        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::Provider(ProviderError::ModelNotFound {
                provider: "anthropic".to_string(),
                model: self.model.clone(),
            }));
        }

        if status.is_server_error() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(Error::Provider(ProviderError::Unavailable {
                provider: "anthropic".to_string(),
                message: format!("Server error {status}: {error_body}"),
            }));
        }

        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            warn!(
                provider = "anthropic",
                status = %status,
                body = %error_body,
                "Unexpected error response"
            );
            return Err(Error::Provider(ProviderError::Request(format!(
                "Anthropic API error {status}: {error_body}"
            ))));
        }

        // Accumulators for building the final response
        let model = self.model.clone();
        let provider_name = "anthropic".to_string();

        // Create the SSE stream
        let stream = response
            .bytes_stream()
            .eventsource()
            .map(
                move |event: std::result::Result<
                    eventsource_stream::Event,
                    eventsource_stream::EventStreamError<reqwest::Error>,
                >| {
                    match event {
                        Ok(event) => {
                            // Parse the SSE data
                            if event.data == "[DONE]" {
                                // End of stream marker (not used by Anthropic but good to handle)
                                return Ok(StreamChunk::Done {
                                    response: CompletionResponse {
                                        id: "streamed".to_string(),
                                        message: Message::assistant(""),
                                        model: model.clone(),
                                        usage: TokenUsage::default(),
                                        provider: provider_name.clone(),
                                        finish_reason: FinishReason::Stop,
                                    },
                                });
                            }

                            let data: Value = match serde_json::from_str(&event.data) {
                                Ok(v) => v,
                                Err(e) => {
                                    return Err(Error::Provider(ProviderError::StreamError {
                                        provider: "anthropic".to_string(),
                                        message: format!("Failed to parse SSE data: {e}"),
                                    }));
                                }
                            };

                            let event_type =
                                data.get("type").and_then(|v| v.as_str()).unwrap_or("");

                            match event_type {
                                "content_block_delta" => {
                                    // Content delta (text or tool_use)
                                    if let Some(delta) = data.get("delta") {
                                        if let Some(text) =
                                            delta.get("text").and_then(|v| v.as_str())
                                        {
                                            return Ok(StreamChunk::ContentDelta {
                                                delta: text.to_string(),
                                            });
                                        }
                                        if let Some(partial_json) =
                                            delta.get("partial_json").and_then(|v| v.as_str())
                                        {
                                            // Tool call argument delta - need to get the block index
                                            if let Some(index) =
                                                data.get("index").and_then(|v| v.as_u64())
                                            {
                                                return Ok(StreamChunk::ToolCallDelta {
                                                    id: format!("tool_{}", index),
                                                    name: None,
                                                    arguments_delta: partial_json.to_string(),
                                                });
                                            }
                                        }
                                    }
                                    Ok(StreamChunk::ContentDelta {
                                        delta: String::new(),
                                    })
                                }
                                "content_block_start" => {
                                    // New content block started (could be tool_use)
                                    if let Some(content_block) = data.get("content_block")
                                        && content_block.get("type").and_then(|v| v.as_str())
                                            == Some("tool_use")
                                    {
                                        let id = content_block
                                            .get("id")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string();
                                        let name = content_block
                                            .get("name")
                                            .and_then(|v| v.as_str())
                                            .map(|s| s.to_string());
                                        return Ok(StreamChunk::ToolCallDelta {
                                            id,
                                            name,
                                            arguments_delta: String::new(),
                                        });
                                    }
                                    Ok(StreamChunk::ContentDelta {
                                        delta: String::new(),
                                    })
                                }
                                "message_delta" => {
                                    // Message-level delta with stop_reason and usage
                                    if let Some(usage) = data.get("usage") {
                                        // Return the final response when we have usage data
                                        let finish_reason = data
                                            .get("delta")
                                            .and_then(|d| d.get("stop_reason"))
                                            .and_then(|v| v.as_str())
                                            .map(|s| match s {
                                                "end_turn" | "stop" => FinishReason::Stop,
                                                "tool_use" => FinishReason::ToolUse,
                                                "max_tokens" => FinishReason::MaxTokens,
                                                _ => FinishReason::Stop,
                                            })
                                            .unwrap_or(FinishReason::Stop);

                                        let prompt_tokens = usage
                                            .get("input_tokens")
                                            .and_then(|v| v.as_u64())
                                            .unwrap_or(0)
                                            as usize;
                                        let completion_tokens = usage
                                            .get("output_tokens")
                                            .and_then(|v| v.as_u64())
                                            .unwrap_or(0)
                                            as usize;

                                        return Ok(StreamChunk::Done {
                                            response: CompletionResponse {
                                                id: "streamed".to_string(),
                                                message: Message::assistant(""),
                                                model: model.clone(),
                                                usage: TokenUsage {
                                                    prompt_tokens,
                                                    completion_tokens,
                                                    total_tokens: prompt_tokens + completion_tokens,
                                                    cost_usd: None,
                                                },
                                                provider: provider_name.clone(),
                                                finish_reason,
                                            },
                                        });
                                    }
                                    Ok(StreamChunk::ContentDelta {
                                        delta: String::new(),
                                    })
                                }
                                "message_stop" => {
                                    // Stream is complete
                                    Ok(StreamChunk::Done {
                                        response: CompletionResponse {
                                            id: "streamed".to_string(),
                                            message: Message::assistant(""),
                                            model: model.clone(),
                                            usage: TokenUsage::default(),
                                            provider: provider_name.clone(),
                                            finish_reason: FinishReason::Stop,
                                        },
                                    })
                                }
                                _ => {
                                    // Ignore other event types (ping, message_start, content_block_stop)
                                    Ok(StreamChunk::ContentDelta {
                                        delta: String::new(),
                                    })
                                }
                            }
                        }
                        Err(e) => Err(Error::Provider(ProviderError::StreamError {
                            provider: "anthropic".to_string(),
                            message: format!("SSE stream error: {e}"),
                        })),
                    }
                },
            )
            .filter(|chunk| {
                // Filter out empty content deltas to reduce noise
                let should_keep =
                    !matches!(chunk, Ok(StreamChunk::ContentDelta { delta }) if delta.is_empty());
                std::future::ready(should_keep)
            });

        Ok(Box::pin(stream))
    }

    fn model_id(&self) -> &str {
        &self.model
    }

    fn max_tokens(&self) -> usize {
        200_000
    }

    fn provider_name(&self) -> &str {
        "anthropic"
    }

    fn supports_strict_tools(&self) -> bool {
        true
    }

    fn supports_streaming_tool_deltas(&self) -> bool {
        true
    }

    fn native_tool_format(&self) -> ToolFormat {
        ToolFormat::Anthropic
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openrustclaw_core::types::ToolDefinition;

    fn make_provider() -> AnthropicProvider {
        AnthropicProvider::new(
            "test-key".to_string(),
            "claude-sonnet-4-20250514".to_string(),
        )
    }

    #[test]
    fn provider_metadata() {
        let p = make_provider();
        assert_eq!(p.provider_name(), "anthropic");
        assert_eq!(p.model_id(), "claude-sonnet-4-20250514");
        assert_eq!(p.max_tokens(), 200_000);
        assert!(p.supports_strict_tools());
        assert!(p.supports_streaming_tool_deltas());
        assert_eq!(p.native_tool_format(), ToolFormat::Anthropic);
    }

    #[test]
    fn build_request_body_basic() {
        let p = make_provider();
        let request = CompletionRequest {
            messages: vec![Message::user("Hello")],
            model: None,
            max_tokens: Some(1024),
            temperature: Some(0.7),
            tools: None,
            system_prompt: Some("You are helpful.".to_string()),
            stream: false,
        };
        let body = p.build_request_body(&request);
        assert_eq!(body["model"], "claude-sonnet-4-20250514");
        assert_eq!(body["max_tokens"], 1024);
        assert_eq!(body["system"], "You are helpful.");
        // f32 temperature gets serialized with f32 precision; compare approximately.
        let temp = body["temperature"].as_f64().unwrap();
        assert!((temp - 0.7).abs() < 0.001, "temperature was {temp}");
        assert!(body.get("tools").is_none());
    }

    #[test]
    fn build_request_body_with_tools() {
        let p = make_provider();
        let request = CompletionRequest {
            messages: vec![Message::user("What is the weather?")],
            model: None,
            max_tokens: None,
            temperature: None,
            tools: Some(vec![ToolDefinition {
                name: "get_weather".to_string(),
                description: "Get weather".to_string(),
                parameters: serde_json::json!({"type": "object"}),
                strict: false,
            }]),
            system_prompt: None,
            stream: false,
        };
        let body = p.build_request_body(&request);
        let tools = body["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["name"], "get_weather");
    }

    #[test]
    fn parse_text_response() {
        let p = make_provider();
        let response_json = serde_json::json!({
            "id": "msg_123",
            "type": "message",
            "role": "assistant",
            "model": "claude-sonnet-4-20250514",
            "content": [{
                "type": "text",
                "text": "Hello! How can I help?"
            }],
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": 10,
                "output_tokens": 8
            }
        });
        let response = p.parse_response(response_json).unwrap();
        assert_eq!(response.id, "msg_123");
        assert_eq!(response.message.content, "Hello! How can I help?");
        assert_eq!(response.finish_reason, FinishReason::Stop);
        assert_eq!(response.usage.prompt_tokens, 10);
        assert_eq!(response.usage.completion_tokens, 8);
        assert_eq!(response.usage.total_tokens, 18);
        assert!(response.message.tool_calls.is_none());
    }

    #[test]
    fn parse_tool_use_response() {
        let p = make_provider();
        let response_json = serde_json::json!({
            "id": "msg_456",
            "type": "message",
            "role": "assistant",
            "model": "claude-sonnet-4-20250514",
            "content": [
                {
                    "type": "text",
                    "text": "Let me check."
                },
                {
                    "type": "tool_use",
                    "id": "toolu_789",
                    "name": "get_weather",
                    "input": { "location": "Paris" }
                }
            ],
            "stop_reason": "tool_use",
            "usage": {
                "input_tokens": 20,
                "output_tokens": 30
            }
        });
        let response = p.parse_response(response_json).unwrap();
        assert_eq!(response.finish_reason, FinishReason::ToolUse);
        assert_eq!(response.message.content, "Let me check.");
        let calls = response.message.tool_calls.unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id, "toolu_789");
        assert_eq!(calls[0].name, "get_weather");
        assert_eq!(calls[0].arguments["location"], "Paris");
    }

    #[test]
    fn message_to_anthropic_tool_result() {
        let p = make_provider();
        let msg = Message::tool("toolu_abc", "The weather is sunny.");
        let result = p.message_to_anthropic(&msg);
        assert_eq!(result["role"], "user");
        let content = result["content"].as_array().unwrap();
        assert_eq!(content[0]["type"], "tool_result");
        assert_eq!(content[0]["tool_use_id"], "toolu_abc");
        assert_eq!(content[0]["content"], "The weather is sunny.");
    }

    // ── Error path tests: verify error variant construction for HTTP status codes ──

    #[test]
    fn error_401_produces_auth_failed() {
        // Simulate the error that would be returned for a 401/403 response
        let err = Error::Provider(ProviderError::AuthFailed {
            provider: "anthropic".to_string(),
            message: r#"{"error":{"type":"authentication_error","message":"invalid x-api-key"}}"#
                .to_string(),
        });
        assert!(matches!(
            err,
            Error::Provider(ProviderError::AuthFailed { .. })
        ));
        let msg = err.to_string();
        assert!(
            msg.contains("anthropic"),
            "Error should name the provider: {}",
            msg
        );
        assert!(
            msg.contains("Authentication failed"),
            "Error should describe auth failure: {}",
            msg
        );
    }

    #[test]
    fn error_429_produces_rate_limited() {
        let err = Error::Provider(ProviderError::RateLimited {
            provider: "anthropic".to_string(),
            retry_after_secs: Some(30),
        });
        assert!(matches!(
            err,
            Error::Provider(ProviderError::RateLimited { .. })
        ));
        let msg = err.to_string();
        assert!(
            msg.contains("Rate limited"),
            "Error should describe rate limiting: {}",
            msg
        );
        assert!(
            msg.contains("30"),
            "Error should include retry_after: {}",
            msg
        );
    }

    #[test]
    fn error_429_without_retry_after() {
        let err = Error::Provider(ProviderError::RateLimited {
            provider: "anthropic".to_string(),
            retry_after_secs: None,
        });
        assert!(matches!(
            err,
            Error::Provider(ProviderError::RateLimited {
                retry_after_secs: None,
                ..
            })
        ));
    }

    #[test]
    fn error_500_produces_unavailable() {
        let err = Error::Provider(ProviderError::Unavailable {
            provider: "anthropic".to_string(),
            message: "Server error 500 Internal Server Error: overloaded".to_string(),
        });
        assert!(matches!(
            err,
            Error::Provider(ProviderError::Unavailable { .. })
        ));
        let msg = err.to_string();
        assert!(
            msg.contains("unavailable"),
            "Error should describe unavailability: {}",
            msg
        );
        assert!(
            msg.contains("overloaded"),
            "Error should include server message: {}",
            msg
        );
    }

    #[test]
    fn error_404_produces_model_not_found() {
        let err = Error::Provider(ProviderError::ModelNotFound {
            provider: "anthropic".to_string(),
            model: "claude-nonexistent".to_string(),
        });
        assert!(matches!(
            err,
            Error::Provider(ProviderError::ModelNotFound { .. })
        ));
        let msg = err.to_string();
        assert!(
            msg.contains("claude-nonexistent"),
            "Error should name the model: {}",
            msg
        );
    }

    #[test]
    fn parse_response_missing_content_returns_empty_message() {
        let p = make_provider();
        // A response with no content array -- should still parse without panic
        let response_json = serde_json::json!({
            "id": "msg_empty",
            "type": "message",
            "role": "assistant",
            "model": "claude-sonnet-4-20250514",
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": 5,
                "output_tokens": 0
            }
        });
        let response = p.parse_response(response_json).unwrap();
        assert_eq!(response.message.content, "");
        assert!(response.message.tool_calls.is_none());
    }

    #[test]
    fn parse_response_missing_usage_returns_default() {
        let p = make_provider();
        let response_json = serde_json::json!({
            "id": "msg_no_usage",
            "type": "message",
            "role": "assistant",
            "model": "claude-sonnet-4-20250514",
            "content": [{"type": "text", "text": "Hello"}],
            "stop_reason": "end_turn"
        });
        let response = p.parse_response(response_json).unwrap();
        assert_eq!(response.usage.prompt_tokens, 0);
        assert_eq!(response.usage.completion_tokens, 0);
        assert_eq!(response.usage.total_tokens, 0);
    }

    #[test]
    fn parse_response_max_tokens_finish_reason() {
        let p = make_provider();
        let response_json = serde_json::json!({
            "id": "msg_truncated",
            "type": "message",
            "role": "assistant",
            "model": "claude-sonnet-4-20250514",
            "content": [{"type": "text", "text": "Truncated..."}],
            "stop_reason": "max_tokens",
            "usage": { "input_tokens": 100, "output_tokens": 4096 }
        });
        let response = p.parse_response(response_json).unwrap();
        assert_eq!(response.finish_reason, FinishReason::MaxTokens);
    }

    #[test]
    fn build_request_body_filters_system_messages() {
        let p = make_provider();
        let request = CompletionRequest {
            messages: vec![Message::system("Be helpful"), Message::user("Hello")],
            model: None,
            max_tokens: None,
            temperature: None,
            tools: None,
            system_prompt: None,
            stream: false,
        };
        let body = p.build_request_body(&request);
        // System messages should be extracted to top-level "system" field
        let messages = body["messages"].as_array().unwrap();
        for msg in messages {
            assert_ne!(
                msg["role"].as_str().unwrap(),
                "system",
                "System messages should not appear in messages array"
            );
        }
        // The system content should appear in the top-level "system" field
        assert_eq!(body["system"], "Be helpful");
    }
}
