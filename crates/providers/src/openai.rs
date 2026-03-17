//! OpenAI Chat Completions API provider.
//!
//! Implements the [`LlmProvider`] trait by calling the OpenAI Chat Completions
//! API (`POST /v1/chat/completions`) via `reqwest`.

use std::pin::Pin;

use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures::{Stream, StreamExt};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde_json::Value;
use secrecy::{ExposeSecret, SecretString};
use tracing::{debug, warn};

use openrustclaw_core::error::{Error, ProviderError, Result};
use openrustclaw_core::traits::LlmProvider;
use openrustclaw_core::types::{
    CompletionRequest, CompletionResponse, FinishReason, Message, Role, StreamChunk, TokenUsage,
    ToolFormat,
};

use crate::tool_formats::{parse_openai_tool_calls, translate_tool_definition};

/// Default base URL for the OpenAI API.
const DEFAULT_BASE_URL: &str = "https://api.openai.com";

/// LLM provider implementation for OpenAI's Chat Completions API.
pub struct OpenAiProvider {
    client: reqwest::Client,
    api_key: SecretString,
    model: String,
    base_url: String,
}

impl OpenAiProvider {
    /// Create a new OpenAI provider with the given API key and model.
    pub fn new(api_key: impl Into<SecretString>, model: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: api_key.into(),
            model,
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }

    /// Create a new OpenAI provider with a custom base URL.
    pub fn with_base_url(api_key: impl Into<SecretString>, model: String, base_url: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: api_key.into(),
            model,
            base_url,
        }
    }

    /// Build the request body for the OpenAI Chat Completions API.
    fn build_request_body(&self, request: &CompletionRequest) -> Value {
        let model = request
            .model
            .as_deref()
            .unwrap_or(&self.model);

        // Build the messages array.
        let mut messages: Vec<Value> = Vec::new();

        // Add system prompt as the first message if present.
        if let Some(ref system_prompt) = request.system_prompt {
            messages.push(serde_json::json!({
                "role": "system",
                "content": system_prompt,
            }));
        }

        for msg in &request.messages {
            messages.push(self.message_to_openai(msg));
        }

        let mut body = serde_json::json!({
            "model": model,
            "messages": messages,
        });

        if let Some(max_tokens) = request.max_tokens {
            body["max_tokens"] = serde_json::json!(max_tokens);
        }

        if let Some(temp) = request.temperature {
            body["temperature"] = serde_json::json!(temp);
        }

        if let Some(tools) = &request.tools {
            if !tools.is_empty() {
                let tool_defs: Vec<Value> = tools
                    .iter()
                    .map(|t| translate_tool_definition(t, ToolFormat::OpenAi))
                    .collect();
                body["tools"] = Value::Array(tool_defs);
            }
        }

        body
    }

    /// Convert a unified [`Message`] to the OpenAI message format.
    fn message_to_openai(&self, msg: &Message) -> Value {
        let role = match msg.role {
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::System => "system",
            Role::Tool => "tool",
        };

        // Handle tool result messages.
        if msg.role == Role::Tool {
            return serde_json::json!({
                "role": "tool",
                "content": msg.content,
                "tool_call_id": msg.tool_call_id.as_deref().unwrap_or(""),
            });
        }

        // Handle assistant messages with tool calls.
        if msg.role == Role::Assistant {
            if let Some(ref tool_calls) = msg.tool_calls {
                if !tool_calls.is_empty() {
                    let tc_values: Vec<Value> = tool_calls
                        .iter()
                        .map(|tc| {
                            serde_json::json!({
                                "id": tc.id,
                                "type": "function",
                                "function": {
                                    "name": tc.name,
                                    "arguments": serde_json::to_string(&tc.arguments)
                                        .unwrap_or_else(|_| "{}".to_string()),
                                }
                            })
                        })
                        .collect();

                    let mut result = serde_json::json!({
                        "role": "assistant",
                        "tool_calls": tc_values,
                    });

                    if !msg.content.is_empty() {
                        result["content"] = Value::String(msg.content.clone());
                    }

                    return result;
                }
            }
        }

        serde_json::json!({
            "role": role,
            "content": msg.content,
        })
    }

    /// Parse the OpenAI API response JSON into a [`CompletionResponse`].
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

        // Get the first choice.
        let choice = body
            .get("choices")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .ok_or_else(|| {
                Error::Provider(ProviderError::Parse(
                    "No choices in OpenAI response".to_string(),
                ))
            })?;

        let finish_reason_str = choice
            .get("finish_reason")
            .and_then(|v| v.as_str())
            .unwrap_or("stop");

        let finish_reason = match finish_reason_str {
            "stop" => FinishReason::Stop,
            "tool_calls" => FinishReason::ToolUse,
            "length" => FinishReason::MaxTokens,
            "content_filter" => FinishReason::ContentFilter,
            _ => FinishReason::Stop,
        };

        let assistant_msg = choice
            .get("message")
            .ok_or_else(|| {
                Error::Provider(ProviderError::Parse(
                    "No message in OpenAI choice".to_string(),
                ))
            })?;

        let text_content = assistant_msg
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Parse tool calls if present.
        let tool_calls = assistant_msg
            .get("tool_calls")
            .and_then(|v| v.as_array())
            .map(|arr| parse_openai_tool_calls(arr))
            .transpose()?;

        let mut message = Message::assistant(text_content);
        if let Some(ref calls) = tool_calls {
            if !calls.is_empty() {
                message.tool_calls = Some(calls.clone());
            }
        }

        // Parse usage.
        let usage = if let Some(usage_obj) = body.get("usage") {
            TokenUsage {
                prompt_tokens: usage_obj
                    .get("prompt_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as usize,
                completion_tokens: usage_obj
                    .get("completion_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as usize,
                total_tokens: usage_obj
                    .get("total_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as usize,
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
            provider: "openai".to_string(),
            finish_reason,
        })
    }

    /// Build default headers for OpenAI API requests.
    fn default_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", self.api_key.expose_secret()))
                .unwrap_or_else(|_| HeaderValue::from_static("")),
        );
        headers
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let url = format!("{}/v1/chat/completions", self.base_url);
        let body = self.build_request_body(&request);

        debug!(provider = "openai", model = %self.model, "Sending completion request");

        let response = self
            .client
            .post(&url)
            .headers(self.default_headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                Error::Provider(ProviderError::Request(format!(
                    "OpenAI request failed: {e}"
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
                provider: "openai".to_string(),
                retry_after_secs: retry_after,
            }));
        }

        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            let error_body = response.text().await.unwrap_or_default();
            return Err(Error::Provider(ProviderError::AuthFailed {
                provider: "openai".to_string(),
                message: error_body,
            }));
        }

        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::Provider(ProviderError::ModelNotFound {
                provider: "openai".to_string(),
                model: self.model.clone(),
            }));
        }

        if status.is_server_error() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(Error::Provider(ProviderError::Unavailable {
                provider: "openai".to_string(),
                message: format!("Server error {status}: {error_body}"),
            }));
        }

        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            warn!(
                provider = "openai",
                status = %status,
                body = %error_body,
                "Unexpected error response"
            );
            return Err(Error::Provider(ProviderError::Request(format!(
                "OpenAI API error {status}: {error_body}"
            ))));
        }

        let response_body: Value = response.json().await.map_err(|e| {
            Error::Provider(ProviderError::Parse(format!(
                "Failed to parse OpenAI response: {e}"
            )))
        })?;

        self.parse_response(response_body)
    }

    async fn stream(
        &self,
        request: CompletionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>> {
        let url = format!("{}/v1/chat/completions", self.base_url);
        let mut body = self.build_request_body(&request);
        body["stream"] = serde_json::json!(true);

        debug!(provider = "openai", model = %self.model, "Sending streaming completion request");

        let response = self
            .client
            .post(&url)
            .headers(self.default_headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                Error::Provider(ProviderError::Request(format!(
                    "OpenAI streaming request failed: {e}"
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
                provider: "openai".to_string(),
                retry_after_secs: retry_after,
            }));
        }

        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            let error_body = response.text().await.unwrap_or_default();
            return Err(Error::Provider(ProviderError::AuthFailed {
                provider: "openai".to_string(),
                message: error_body,
            }));
        }

        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::Provider(ProviderError::ModelNotFound {
                provider: "openai".to_string(),
                model: self.model.clone(),
            }));
        }

        if status.is_server_error() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(Error::Provider(ProviderError::Unavailable {
                provider: "openai".to_string(),
                message: format!("Server error {status}: {error_body}"),
            }));
        }

        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            warn!(
                provider = "openai",
                status = %status,
                body = %error_body,
                "Unexpected error response"
            );
            return Err(Error::Provider(ProviderError::Request(format!(
                "OpenAI API error {status}: {error_body}"
            ))));
        }

        let model = self.model.clone();
        let provider_name = "openai".to_string();

        // Create the SSE stream
        let stream = response
            .bytes_stream()
            .eventsource()
            .map(move |event: std::result::Result<eventsource_stream::Event, eventsource_stream::EventStreamError<reqwest::Error>>| {
                match event {
                    Ok(event) => {
                        // Check for [DONE] marker
                        if event.data == "[DONE]" {
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
                                    provider: "openai".to_string(),
                                    message: format!("Failed to parse SSE data: {e}"),
                                }));
                            }
                        };

                        // Extract the delta from the first choice
                        let choice = data
                            .get("choices")
                            .and_then(|v| v.as_array())
                            .and_then(|arr| arr.first());

                        if let Some(choice) = choice {
                            // Check for finish_reason
                            if let Some(finish_reason) = choice.get("finish_reason").and_then(|v| v.as_str()) {
                                if !finish_reason.is_empty() {
                                    let reason = match finish_reason {
                                        "stop" => FinishReason::Stop,
                                        "tool_calls" => FinishReason::ToolUse,
                                        "length" => FinishReason::MaxTokens,
                                        "content_filter" => FinishReason::ContentFilter,
                                        _ => FinishReason::Stop,
                                    };
                                    return Ok(StreamChunk::Done {
                                        response: CompletionResponse {
                                            id: data.get("id").and_then(|v| v.as_str()).unwrap_or("streamed").to_string(),
                                            message: Message::assistant(""),
                                            model: data.get("model").and_then(|v| v.as_str()).unwrap_or(&model).to_string(),
                                            usage: TokenUsage::default(),
                                            provider: provider_name.clone(),
                                            finish_reason: reason,
                                        },
                                    });
                                }
                            }

                            // Process delta
                            if let Some(delta) = choice.get("delta") {
                                // Check for content delta
                                if let Some(content) = delta.get("content").and_then(|v| v.as_str()) {
                                    if !content.is_empty() {
                                        return Ok(StreamChunk::ContentDelta {
                                            delta: content.to_string(),
                                        });
                                    }
                                }

                                // Check for tool_calls delta
                                if let Some(tool_calls) = delta.get("tool_calls").and_then(|v| v.as_array()) {
                                    if let Some(tool_call) = tool_calls.first() {
                                        let id = tool_call
                                            .get("id")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string();
                                        let name = tool_call
                                            .get("function")
                                            .and_then(|f| f.get("name"))
                                            .and_then(|v| v.as_str())
                                            .map(|s| s.to_string());
                                        let arguments = tool_call
                                            .get("function")
                                            .and_then(|f| f.get("arguments"))
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string();

                                        if !id.is_empty() || name.is_some() || !arguments.is_empty() {
                                            return Ok(StreamChunk::ToolCallDelta {
                                                id,
                                                name,
                                                arguments_delta: arguments,
                                            });
                                        }
                                    }
                                }
                            }
                        }

                        // Empty delta - return a no-op chunk that will be filtered
                        Ok(StreamChunk::ContentDelta { delta: String::new() })
                    }
                    Err(e) => Err(Error::Provider(ProviderError::StreamError {
                        provider: "openai".to_string(),
                        message: format!("SSE stream error: {e}"),
                    })),
                }
            })
            .filter(|chunk| {
                // Filter out empty content deltas to reduce noise
                let should_keep = match chunk {
                    Ok(StreamChunk::ContentDelta { delta }) if delta.is_empty() => false,
                    _ => true,
                };
                std::future::ready(should_keep)
            });

        Ok(Box::pin(stream))
    }

    fn model_id(&self) -> &str {
        &self.model
    }

    fn max_tokens(&self) -> usize {
        128_000
    }

    fn provider_name(&self) -> &str {
        "openai"
    }

    fn supports_strict_tools(&self) -> bool {
        true
    }

    fn supports_streaming_tool_deltas(&self) -> bool {
        false
    }

    fn native_tool_format(&self) -> ToolFormat {
        ToolFormat::OpenAi
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openrustclaw_core::types::ToolDefinition;

    fn make_provider() -> OpenAiProvider {
        OpenAiProvider::new("test-key".to_string(), "gpt-4o".to_string())
    }

    #[test]
    fn provider_metadata() {
        let p = make_provider();
        assert_eq!(p.provider_name(), "openai");
        assert_eq!(p.model_id(), "gpt-4o");
        assert_eq!(p.max_tokens(), 128_000);
        assert!(p.supports_strict_tools());
        assert!(!p.supports_streaming_tool_deltas());
        assert_eq!(p.native_tool_format(), ToolFormat::OpenAi);
    }

    #[test]
    fn build_request_body_basic() {
        let p = make_provider();
        let request = CompletionRequest {
            messages: vec![Message::user("Hello")],
            model: None,
            max_tokens: Some(1024),
            temperature: Some(0.5),
            tools: None,
            system_prompt: Some("Be helpful.".to_string()),
            stream: false,
        };
        let body = p.build_request_body(&request);
        assert_eq!(body["model"], "gpt-4o");
        assert_eq!(body["max_tokens"], 1024);
        // System prompt should be the first message.
        let messages = body["messages"].as_array().unwrap();
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(messages[0]["content"], "Be helpful.");
        assert_eq!(messages[1]["role"], "user");
        assert_eq!(messages[1]["content"], "Hello");
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
                strict: true,
            }]),
            system_prompt: None,
            stream: false,
        };
        let body = p.build_request_body(&request);
        let tools = body["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["type"], "function");
        assert_eq!(tools[0]["function"]["name"], "get_weather");
        assert_eq!(tools[0]["function"]["strict"], true);
    }

    #[test]
    fn parse_text_response() {
        let p = make_provider();
        let response_json = serde_json::json!({
            "id": "chatcmpl-abc",
            "object": "chat.completion",
            "model": "gpt-4o",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "Hello! How can I help?"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 8,
                "total_tokens": 18
            }
        });
        let response = p.parse_response(response_json).unwrap();
        assert_eq!(response.id, "chatcmpl-abc");
        assert_eq!(response.message.content, "Hello! How can I help?");
        assert_eq!(response.finish_reason, FinishReason::Stop);
        assert_eq!(response.usage.prompt_tokens, 10);
        assert_eq!(response.usage.completion_tokens, 8);
        assert_eq!(response.usage.total_tokens, 18);
        assert!(response.message.tool_calls.is_none());
    }

    #[test]
    fn parse_tool_call_response() {
        let p = make_provider();
        let response_json = serde_json::json!({
            "id": "chatcmpl-xyz",
            "model": "gpt-4o",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_123",
                        "type": "function",
                        "function": {
                            "name": "get_weather",
                            "arguments": "{\"location\":\"Tokyo\"}"
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }],
            "usage": {
                "prompt_tokens": 15,
                "completion_tokens": 20,
                "total_tokens": 35
            }
        });
        let response = p.parse_response(response_json).unwrap();
        assert_eq!(response.finish_reason, FinishReason::ToolUse);
        let calls = response.message.tool_calls.unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id, "call_123");
        assert_eq!(calls[0].name, "get_weather");
        assert_eq!(calls[0].arguments["location"], "Tokyo");
    }

    #[test]
    fn message_to_openai_tool_result() {
        let p = make_provider();
        let msg = Message::tool("call_abc", "The weather is cloudy.");
        let result = p.message_to_openai(&msg);
        assert_eq!(result["role"], "tool");
        assert_eq!(result["content"], "The weather is cloudy.");
        assert_eq!(result["tool_call_id"], "call_abc");
    }
}
