//! OpenRouter API provider.
//!
//! Implements the [`LlmProvider`] trait by calling the OpenRouter API, which is
//! OpenAI-compatible (`POST /api/v1/chat/completions`) with extra routing
//! features like `:floor`, `:nitro`, and `:online` suffixes.

use std::pin::Pin;

use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures::{Stream, StreamExt};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{debug, warn};

use openrustclaw_core::error::{Error, ProviderError, Result};
use openrustclaw_core::traits::LlmProvider;
use openrustclaw_core::types::{
    CompletionRequest, CompletionResponse, FinishReason, Message, Role, StreamChunk, TokenUsage,
    ToolFormat,
};

use crate::tool_formats::{parse_openai_tool_calls, translate_tool_definition};

/// Default base URL for the OpenRouter API.
const DEFAULT_BASE_URL: &str = "https://openrouter.ai/api";

/// HTTP Referer header sent to OpenRouter for tracking.
const REFERER: &str = "https://openrustclaw.dev";

/// Routing strategy for OpenRouter model selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteStrategy {
    /// Optimize for lowest price (`:floor` suffix).
    Price,
    /// Optimize for highest throughput (`:nitro` suffix).
    Throughput,
    /// Default quality routing (no suffix).
    Quality,
    /// Include web search results (`:online` suffix).
    WebSearch,
}

impl RouteStrategy {
    /// Return the model suffix for this routing strategy, if any.
    fn suffix(&self) -> &str {
        match self {
            RouteStrategy::Price => ":floor",
            RouteStrategy::Throughput => ":nitro",
            RouteStrategy::Quality => "",
            RouteStrategy::WebSearch => ":online",
        }
    }
}

impl std::fmt::Display for RouteStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RouteStrategy::Price => write!(f, "price"),
            RouteStrategy::Throughput => write!(f, "throughput"),
            RouteStrategy::Quality => write!(f, "quality"),
            RouteStrategy::WebSearch => write!(f, "web_search"),
        }
    }
}

/// LLM provider implementation for the OpenRouter API.
///
/// OpenRouter is OpenAI-compatible but supports automatic model routing and
/// multiple providers behind a single API key.
pub struct OpenRouterProvider {
    client: reqwest::Client,
    api_key: SecretString,
    model: String,
    route_strategy: RouteStrategy,
    base_url: String,
}

impl OpenRouterProvider {
    /// Create a new OpenRouter provider with the given API key and model.
    ///
    /// Uses the default quality routing strategy.
    pub fn new(api_key: impl Into<SecretString>, model: String) -> Self {
        Self {
            client: reqwest::Client::builder()
                .connect_timeout(std::time::Duration::from_secs(10))
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("Failed to build HTTP client"),
            api_key: api_key.into(),
            model,
            route_strategy: RouteStrategy::Quality,
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }

    /// Create a new OpenRouter provider with a specific routing strategy.
    pub fn with_strategy(
        api_key: impl Into<SecretString>,
        model: String,
        strategy: RouteStrategy,
    ) -> Self {
        Self {
            client: reqwest::Client::builder()
                .connect_timeout(std::time::Duration::from_secs(10))
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("Failed to build HTTP client"),
            api_key: api_key.into(),
            model,
            route_strategy: strategy,
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }

    /// Return the effective model string with any routing suffix applied.
    fn effective_model(&self, model_override: Option<&str>) -> String {
        let base = model_override.unwrap_or(&self.model);
        let suffix = self.route_strategy.suffix();
        if suffix.is_empty() {
            base.to_string()
        } else {
            format!("{base}{suffix}")
        }
    }

    /// Build the request body (OpenAI-compatible format).
    fn build_request_body(&self, request: &CompletionRequest) -> Value {
        let model = self.effective_model(request.model.as_deref());

        let mut messages: Vec<Value> = Vec::new();

        // Add system prompt as first message if present.
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

        if let Some(tools) = &request.tools
            && !tools.is_empty()
        {
            let tool_defs: Vec<Value> = tools
                .iter()
                .map(|t| translate_tool_definition(t, ToolFormat::OpenAi))
                .collect();
            body["tools"] = Value::Array(tool_defs);
        }

        body
    }

    /// Convert a unified [`Message`] to the OpenAI message format.
    ///
    /// This is the same format as OpenAI since OpenRouter is OpenAI-compatible.
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
        if msg.role == Role::Assistant
            && let Some(ref tool_calls) = msg.tool_calls
            && !tool_calls.is_empty()
        {
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

        serde_json::json!({
            "role": role,
            "content": msg.content,
        })
    }

    /// Parse the OpenRouter (OpenAI-compatible) response.
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

        let choice = body
            .get("choices")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .ok_or_else(|| {
                Error::Provider(ProviderError::Parse(
                    "No choices in OpenRouter response".to_string(),
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

        let assistant_msg = choice.get("message").ok_or_else(|| {
            Error::Provider(ProviderError::Parse(
                "No message in OpenRouter choice".to_string(),
            ))
        })?;

        let text_content = assistant_msg
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let tool_calls = assistant_msg
            .get("tool_calls")
            .and_then(|v| v.as_array())
            .map(|arr| parse_openai_tool_calls(arr))
            .transpose()?;

        let mut message = Message::assistant(text_content);
        if let Some(ref calls) = tool_calls
            && !calls.is_empty()
        {
            message.tool_calls = Some(calls.clone());
        }

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
            provider: "openrouter".to_string(),
            finish_reason,
        })
    }

    /// Build default headers for OpenRouter API requests.
    fn default_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", self.api_key.expose_secret()))
                .unwrap_or_else(|_| HeaderValue::from_static("")),
        );
        headers.insert("HTTP-Referer", HeaderValue::from_static(REFERER));
        headers
    }
}

#[async_trait]
impl LlmProvider for OpenRouterProvider {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let url = format!("{}/v1/chat/completions", self.base_url);
        let body = self.build_request_body(&request);

        debug!(
            provider = "openrouter",
            model = %self.model,
            strategy = %self.route_strategy,
            "Sending completion request"
        );

        let response = self
            .client
            .post(&url)
            .headers(self.default_headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                Error::Provider(ProviderError::Request(format!(
                    "OpenRouter request failed: {e}"
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
                provider: "openrouter".to_string(),
                retry_after_secs: retry_after,
            }));
        }

        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            let error_body = response.text().await.unwrap_or_default();
            return Err(Error::Provider(ProviderError::AuthFailed {
                provider: "openrouter".to_string(),
                message: error_body,
            }));
        }

        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::Provider(ProviderError::ModelNotFound {
                provider: "openrouter".to_string(),
                model: self.model.clone(),
            }));
        }

        if status.is_server_error() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(Error::Provider(ProviderError::Unavailable {
                provider: "openrouter".to_string(),
                message: format!("Server error {status}: {error_body}"),
            }));
        }

        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            warn!(
                provider = "openrouter",
                status = %status,
                body = %error_body,
                "Unexpected error response"
            );
            return Err(Error::Provider(ProviderError::Request(format!(
                "OpenRouter API error {status}: {error_body}"
            ))));
        }

        let response_body: Value = response.json().await.map_err(|e| {
            Error::Provider(ProviderError::Parse(format!(
                "Failed to parse OpenRouter response: {e}"
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

        debug!(
            provider = "openrouter",
            model = %self.model,
            strategy = %self.route_strategy,
            "Sending streaming completion request"
        );

        let response = self
            .client
            .post(&url)
            .headers(self.default_headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                Error::Provider(ProviderError::Request(format!(
                    "OpenRouter streaming request failed: {e}"
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
                provider: "openrouter".to_string(),
                retry_after_secs: retry_after,
            }));
        }

        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            let error_body = response.text().await.unwrap_or_default();
            return Err(Error::Provider(ProviderError::AuthFailed {
                provider: "openrouter".to_string(),
                message: error_body,
            }));
        }

        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::Provider(ProviderError::ModelNotFound {
                provider: "openrouter".to_string(),
                model: self.model.clone(),
            }));
        }

        if status.is_server_error() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(Error::Provider(ProviderError::Unavailable {
                provider: "openrouter".to_string(),
                message: format!("Server error {status}: {error_body}"),
            }));
        }

        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            warn!(
                provider = "openrouter",
                status = %status,
                body = %error_body,
                "Unexpected error response"
            );
            return Err(Error::Provider(ProviderError::Request(format!(
                "OpenRouter API error {status}: {error_body}"
            ))));
        }

        let model = self.model.clone();
        let provider_name = "openrouter".to_string();

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
                                        provider: "openrouter".to_string(),
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
                                if let Some(finish_reason) =
                                    choice.get("finish_reason").and_then(|v| v.as_str())
                                    && !finish_reason.is_empty()
                                {
                                    let reason = match finish_reason {
                                        "stop" => FinishReason::Stop,
                                        "tool_calls" => FinishReason::ToolUse,
                                        "length" => FinishReason::MaxTokens,
                                        "content_filter" => FinishReason::ContentFilter,
                                        _ => FinishReason::Stop,
                                    };
                                    return Ok(StreamChunk::Done {
                                        response: CompletionResponse {
                                            id: data
                                                .get("id")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or("streamed")
                                                .to_string(),
                                            message: Message::assistant(""),
                                            model: data
                                                .get("model")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or(&model)
                                                .to_string(),
                                            usage: TokenUsage::default(),
                                            provider: provider_name.clone(),
                                            finish_reason: reason,
                                        },
                                    });
                                }

                                // Process delta
                                if let Some(delta) = choice.get("delta") {
                                    // Check for content delta
                                    if let Some(content) =
                                        delta.get("content").and_then(|v| v.as_str())
                                        && !content.is_empty()
                                    {
                                        return Ok(StreamChunk::ContentDelta {
                                            delta: content.to_string(),
                                        });
                                    }

                                    // Check for tool_calls delta
                                    if let Some(tool_calls) =
                                        delta.get("tool_calls").and_then(|v| v.as_array())
                                        && let Some(tool_call) = tool_calls.first()
                                    {
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

                                        if !id.is_empty() || name.is_some() || !arguments.is_empty()
                                        {
                                            return Ok(StreamChunk::ToolCallDelta {
                                                id,
                                                name,
                                                arguments_delta: arguments,
                                            });
                                        }
                                    }
                                }
                            }

                            // Empty delta - return a no-op chunk that will be filtered
                            Ok(StreamChunk::ContentDelta {
                                delta: String::new(),
                            })
                        }
                        Err(e) => Err(Error::Provider(ProviderError::StreamError {
                            provider: "openrouter".to_string(),
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
        // OpenRouter supports many models; use a conservative default.
        128_000
    }

    fn provider_name(&self) -> &str {
        "openrouter"
    }

    fn supports_strict_tools(&self) -> bool {
        // Depends on the underlying model, but generally yes via OpenAI format.
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

    fn make_provider() -> OpenRouterProvider {
        OpenRouterProvider::new(
            "test-key".to_string(),
            "anthropic/claude-sonnet-4-20250514".to_string(),
        )
    }

    #[test]
    fn provider_metadata() {
        let p = make_provider();
        assert_eq!(p.provider_name(), "openrouter");
        assert_eq!(p.model_id(), "anthropic/claude-sonnet-4-20250514");
        assert_eq!(p.max_tokens(), 128_000);
        assert_eq!(p.native_tool_format(), ToolFormat::OpenAi);
    }

    #[test]
    fn route_strategy_suffix() {
        assert_eq!(RouteStrategy::Price.suffix(), ":floor");
        assert_eq!(RouteStrategy::Throughput.suffix(), ":nitro");
        assert_eq!(RouteStrategy::Quality.suffix(), "");
        assert_eq!(RouteStrategy::WebSearch.suffix(), ":online");
    }

    #[test]
    fn effective_model_with_strategy() {
        let p = OpenRouterProvider::with_strategy(
            "key".to_string(),
            "openai/gpt-4o".to_string(),
            RouteStrategy::Price,
        );
        assert_eq!(p.effective_model(None), "openai/gpt-4o:floor");
    }

    #[test]
    fn effective_model_with_override() {
        let p = make_provider();
        assert_eq!(
            p.effective_model(Some("google/gemini-pro")),
            "google/gemini-pro"
        );
    }

    #[test]
    fn build_request_body_basic() {
        let p = make_provider();
        let request = CompletionRequest {
            messages: vec![Message::user("Hello")],
            model: None,
            max_tokens: Some(1024),
            temperature: None,
            tools: None,
            system_prompt: None,
            stream: false,
        };
        let body = p.build_request_body(&request);
        assert_eq!(body["model"], "anthropic/claude-sonnet-4-20250514");
        assert_eq!(body["max_tokens"], 1024);
    }

    #[test]
    fn parse_response_basic() {
        let p = make_provider();
        let response_json = serde_json::json!({
            "id": "gen-abc",
            "model": "anthropic/claude-sonnet-4-20250514",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "Hello from OpenRouter!"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 5,
                "completion_tokens": 10,
                "total_tokens": 15
            }
        });
        let response = p.parse_response(response_json).unwrap();
        assert_eq!(response.provider, "openrouter");
        assert_eq!(response.message.content, "Hello from OpenRouter!");
        assert_eq!(response.finish_reason, FinishReason::Stop);
    }
}
