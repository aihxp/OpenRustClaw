//! Local Ollama provider.
//!
//! Implements the [`LlmProvider`] trait by calling the Ollama REST API
//! (`POST /api/chat`). Ollama runs local models and requires no API key.

use std::pin::Pin;

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue};
use serde_json::Value;
use tracing::{debug, warn};

use openrustclaw_core::error::{Error, ProviderError, Result};
use openrustclaw_core::traits::LlmProvider;
use openrustclaw_core::types::{
    CompletionRequest, CompletionResponse, FinishReason, Message, Role, StreamChunk, TokenUsage,
    ToolFormat,
};

use crate::tool_formats::translate_tool_definition;

/// Default base URL for the local Ollama instance.
const DEFAULT_BASE_URL: &str = "http://localhost:11434";

/// LLM provider implementation for local Ollama models.
///
/// Ollama runs locally and has no API key requirement. It uses a slightly
/// different request format from OpenAI: `model`, `messages`, `tools`,
/// `stream: false`, and `options` for parameters like temperature.
pub struct OllamaProvider {
    client: reqwest::Client,
    model: String,
    base_url: String,
}

impl OllamaProvider {
    /// Create a new Ollama provider with the given model name.
    ///
    /// Uses the default base URL (`http://localhost:11434`).
    pub fn new(model: String) -> Self {
        Self {
            client: reqwest::Client::builder()
                .connect_timeout(std::time::Duration::from_secs(10))
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("Failed to build HTTP client"),
            model,
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }

    /// Create a new Ollama provider with a custom base URL.
    pub fn with_base_url(model: String, base_url: String) -> Self {
        Self {
            client: reqwest::Client::builder()
                .connect_timeout(std::time::Duration::from_secs(10))
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("Failed to build HTTP client"),
            model,
            base_url,
        }
    }

    /// Build the request body for the Ollama `/api/chat` endpoint.
    ///
    /// Ollama uses a format similar to but distinct from OpenAI:
    /// - `model`: model name
    /// - `messages`: array of `{role, content}` objects
    /// - `tools`: tool definitions (OpenAI-like function format)
    /// - `stream`: always `false` for non-streaming
    /// - `options`: object with parameters like `temperature`, `num_predict`
    fn build_request_body(&self, request: &CompletionRequest) -> Value {
        let model = request.model.as_deref().unwrap_or(&self.model);

        let mut messages: Vec<Value> = Vec::new();

        // Add system prompt as a system message.
        if let Some(ref system_prompt) = request.system_prompt {
            messages.push(serde_json::json!({
                "role": "system",
                "content": system_prompt,
            }));
        }

        for msg in &request.messages {
            messages.push(self.message_to_ollama(msg));
        }

        let mut body = serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": false,
        });

        // Build options object for temperature, max tokens, etc.
        let mut options = serde_json::Map::new();
        if let Some(temp) = request.temperature {
            options.insert("temperature".to_string(), serde_json::json!(temp));
        }
        if let Some(max_tokens) = request.max_tokens {
            options.insert("num_predict".to_string(), serde_json::json!(max_tokens));
        }
        if !options.is_empty() {
            body["options"] = Value::Object(options);
        }

        // Add tools (Ollama supports OpenAI-format tool definitions).
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

    /// Convert a unified [`Message`] to the Ollama message format.
    fn message_to_ollama(&self, msg: &Message) -> Value {
        let role = match msg.role {
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::System => "system",
            Role::Tool => "tool",
        };

        // Handle assistant messages with tool calls.
        if msg.role == Role::Assistant
            && let Some(ref tool_calls) = msg.tool_calls
            && !tool_calls.is_empty()
        {
            let tc_values: Vec<Value> = tool_calls
                .iter()
                .map(|tc| {
                    serde_json::json!({
                        "function": {
                            "name": tc.name,
                            "arguments": tc.arguments,
                        }
                    })
                })
                .collect();

            let mut result = serde_json::json!({
                "role": "assistant",
                "content": msg.content,
                "tool_calls": tc_values,
            });

            // Ollama may expect empty content as empty string.
            if msg.content.is_empty() {
                result["content"] = Value::String(String::new());
            }

            return result;
        }

        serde_json::json!({
            "role": role,
            "content": msg.content,
        })
    }

    /// Parse the Ollama API response JSON into a [`CompletionResponse`].
    fn parse_response(&self, body: Value) -> Result<CompletionResponse> {
        let model = body
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or(&self.model)
            .to_string();

        let message_obj = body.get("message").ok_or_else(|| {
            Error::Provider(ProviderError::Parse(
                "No message in Ollama response".to_string(),
            ))
        })?;

        let text_content = message_obj
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Parse tool calls from the Ollama response.
        // Ollama returns tool calls in a slightly different format:
        // "tool_calls": [{"function": {"name": "...", "arguments": {...}}}]
        let tool_calls = message_obj
            .get("tool_calls")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .enumerate()
                    .filter_map(|(i, tc)| {
                        let function = tc.get("function")?;
                        let name = function.get("name")?.as_str()?.to_string();
                        let arguments = function.get("arguments").cloned().unwrap_or(Value::Null);
                        Some(openrustclaw_core::types::ToolCall {
                            id: format!("ollama_call_{i}"),
                            name,
                            arguments,
                        })
                    })
                    .collect::<Vec<_>>()
            });

        let has_tool_calls = tool_calls.as_ref().is_some_and(|calls| !calls.is_empty());

        let mut message = Message::assistant(text_content);
        if let Some(ref calls) = tool_calls
            && !calls.is_empty()
        {
            message.tool_calls = Some(calls.clone());
        }

        let finish_reason = if has_tool_calls {
            FinishReason::ToolUse
        } else if body.get("done").and_then(|v| v.as_bool()).unwrap_or(true) {
            FinishReason::Stop
        } else {
            FinishReason::MaxTokens
        };

        // Ollama provides some token metrics.
        let prompt_tokens = body
            .get("prompt_eval_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;
        let completion_tokens =
            body.get("eval_count").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

        let usage = TokenUsage {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
            cost_usd: None, // Local models have no API cost.
        };

        // Ollama does not return a request ID, so we generate a placeholder.
        let id = format!(
            "ollama-{}",
            body.get("created_at")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
        );

        Ok(CompletionResponse {
            id,
            message,
            model,
            usage,
            provider: "ollama".to_string(),
            finish_reason,
        })
    }

    /// Build default headers for Ollama API requests.
    fn default_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let url = format!("{}/api/chat", self.base_url);
        let body = self.build_request_body(&request);

        debug!(provider = "ollama", model = %self.model, "Sending completion request");

        let response = self
            .client
            .post(&url)
            .headers(self.default_headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                // Connection errors likely mean Ollama isn't running.
                Error::Provider(ProviderError::Unavailable {
                    provider: "ollama".to_string(),
                    message: format!("Ollama connection failed (is it running?): {e}"),
                })
            })?;

        let status = response.status();

        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::Provider(ProviderError::ModelNotFound {
                provider: "ollama".to_string(),
                model: self.model.clone(),
            }));
        }

        if status.is_server_error() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(Error::Provider(ProviderError::Unavailable {
                provider: "ollama".to_string(),
                message: format!("Server error {status}: {error_body}"),
            }));
        }

        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            warn!(
                provider = "ollama",
                status = %status,
                body = %error_body,
                "Unexpected error response"
            );
            return Err(Error::Provider(ProviderError::Request(format!(
                "Ollama API error {status}: {error_body}"
            ))));
        }

        let response_body: Value = response.json().await.map_err(|e| {
            Error::Provider(ProviderError::Parse(format!(
                "Failed to parse Ollama response: {e}"
            )))
        })?;

        self.parse_response(response_body)
    }

    async fn stream(
        &self,
        request: CompletionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>> {
        let url = format!("{}/api/chat", self.base_url);
        let mut body = self.build_request_body(&request);
        body["stream"] = serde_json::json!(true);

        debug!(provider = "ollama", model = %self.model, "Sending streaming completion request");

        let response = self
            .client
            .post(&url)
            .headers(self.default_headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                // Connection errors likely mean Ollama isn't running.
                Error::Provider(ProviderError::Unavailable {
                    provider: "ollama".to_string(),
                    message: format!("Ollama connection failed (is it running?): {e}"),
                })
            })?;

        let status = response.status();

        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::Provider(ProviderError::ModelNotFound {
                provider: "ollama".to_string(),
                model: self.model.clone(),
            }));
        }

        if status.is_server_error() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(Error::Provider(ProviderError::Unavailable {
                provider: "ollama".to_string(),
                message: format!("Server error {status}: {error_body}"),
            }));
        }

        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            warn!(
                provider = "ollama",
                status = %status,
                body = %error_body,
                "Unexpected error response"
            );
            return Err(Error::Provider(ProviderError::Request(format!(
                "Ollama API error {status}: {error_body}"
            ))));
        }

        let model = self.model.clone();
        let provider_name = "ollama".to_string();

        // Ollama uses NDJSON (newline-delimited JSON) for streaming
        let stream = response
            .bytes_stream()
            .map(move |bytes| {
                match bytes {
                    Ok(bytes) => {
                        // Split by newlines to get individual JSON objects
                        let text = String::from_utf8_lossy(&bytes);
                        let lines: Vec<&str> = text.lines().collect();
                        let results: Vec<Result<StreamChunk>> = lines
                            .into_iter()
                            .filter(|line| !line.is_empty())
                            .map(|line| {
                                let data: Value = match serde_json::from_str(line) {
                                    Ok(v) => v,
                                    Err(e) => {
                                        return Err(Error::Provider(ProviderError::StreamError {
                                            provider: "ollama".to_string(),
                                            message: format!("Failed to parse NDJSON: {e}"),
                                        }));
                                    }
                                };

                                // Check if stream is done
                                let done =
                                    data.get("done").and_then(|v| v.as_bool()).unwrap_or(false);

                                if done {
                                    // Extract usage statistics if available
                                    let prompt_tokens = data
                                        .get("prompt_eval_count")
                                        .and_then(|v| v.as_u64())
                                        .unwrap_or(0)
                                        as usize;
                                    let completion_tokens = data
                                        .get("eval_count")
                                        .and_then(|v| v.as_u64())
                                        .unwrap_or(0)
                                        as usize;

                                    // Check for tool calls in the final message
                                    let message_obj = data.get("message");
                                    let tool_calls = message_obj
                                        .and_then(|m| m.get("tool_calls"))
                                        .and_then(|v| v.as_array());

                                    let has_tool_calls =
                                        tool_calls.as_ref().is_some_and(|calls| !calls.is_empty());

                                    let finish_reason = if has_tool_calls {
                                        FinishReason::ToolUse
                                    } else {
                                        FinishReason::Stop
                                    };

                                    return Ok(StreamChunk::Done {
                                        response: CompletionResponse {
                                            id: format!(
                                                "ollama-{}",
                                                data.get("created_at")
                                                    .and_then(|v| v.as_str())
                                                    .unwrap_or("streamed")
                                            ),
                                            message: Message::assistant(""),
                                            model: data
                                                .get("model")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or(&model)
                                                .to_string(),
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

                                // Extract content delta from message
                                if let Some(message) = data.get("message")
                                    && let Some(content) =
                                        message.get("content").and_then(|v| v.as_str())
                                    && !content.is_empty()
                                {
                                    return Ok(StreamChunk::ContentDelta {
                                        delta: content.to_string(),
                                    });
                                }

                                Ok(StreamChunk::ContentDelta {
                                    delta: String::new(),
                                })
                            })
                            .collect();

                        futures::stream::iter(results)
                    }
                    Err(e) => futures::stream::iter(vec![Err(Error::Provider(
                        ProviderError::StreamError {
                            provider: "ollama".to_string(),
                            message: format!("Stream error: {e}"),
                        },
                    ))]),
                }
            })
            .flatten()
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
        8_192
    }

    fn provider_name(&self) -> &str {
        "ollama"
    }

    fn supports_strict_tools(&self) -> bool {
        false
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

    fn make_provider() -> OllamaProvider {
        OllamaProvider::new("llama3.1".to_string())
    }

    #[test]
    fn provider_metadata() {
        let p = make_provider();
        assert_eq!(p.provider_name(), "ollama");
        assert_eq!(p.model_id(), "llama3.1");
        assert_eq!(p.max_tokens(), 8_192);
        assert!(!p.supports_strict_tools());
        assert!(!p.supports_streaming_tool_deltas());
        assert_eq!(p.native_tool_format(), ToolFormat::OpenAi);
    }

    #[test]
    fn build_request_body_basic() {
        let p = make_provider();
        let request = CompletionRequest {
            messages: vec![Message::user("Hello")],
            model: None,
            max_tokens: Some(512),
            temperature: Some(0.8),
            tools: None,
            system_prompt: Some("Be concise.".to_string()),
            stream: false,
        };
        let body = p.build_request_body(&request);
        assert_eq!(body["model"], "llama3.1");
        assert_eq!(body["stream"], false);
        // f32 temperature gets serialized with f32 precision; compare approximately.
        let temp = body["options"]["temperature"].as_f64().unwrap();
        assert!((temp - 0.8).abs() < 0.001, "temperature was {temp}");
        assert_eq!(body["options"]["num_predict"], 512);
        let messages = body["messages"].as_array().unwrap();
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(messages[0]["content"], "Be concise.");
    }

    #[test]
    fn parse_text_response() {
        let p = make_provider();
        let response_json = serde_json::json!({
            "model": "llama3.1",
            "created_at": "2024-01-01T00:00:00Z",
            "message": {
                "role": "assistant",
                "content": "Hello! I am Llama."
            },
            "done": true,
            "prompt_eval_count": 10,
            "eval_count": 5
        });
        let response = p.parse_response(response_json).unwrap();
        assert_eq!(response.provider, "ollama");
        assert_eq!(response.message.content, "Hello! I am Llama.");
        assert_eq!(response.finish_reason, FinishReason::Stop);
        assert_eq!(response.usage.prompt_tokens, 10);
        assert_eq!(response.usage.completion_tokens, 5);
        assert!(response.message.tool_calls.is_none());
    }

    #[test]
    fn parse_tool_call_response() {
        let p = make_provider();
        let response_json = serde_json::json!({
            "model": "llama3.1",
            "created_at": "2024-01-01T00:00:00Z",
            "message": {
                "role": "assistant",
                "content": "",
                "tool_calls": [{
                    "function": {
                        "name": "get_weather",
                        "arguments": { "location": "Berlin" }
                    }
                }]
            },
            "done": true,
            "prompt_eval_count": 20,
            "eval_count": 10
        });
        let response = p.parse_response(response_json).unwrap();
        assert_eq!(response.finish_reason, FinishReason::ToolUse);
        let calls = response.message.tool_calls.unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "get_weather");
        assert_eq!(calls[0].arguments["location"], "Berlin");
        assert_eq!(calls[0].id, "ollama_call_0");
    }

    #[test]
    fn custom_base_url() {
        let p = OllamaProvider::with_base_url(
            "codellama".to_string(),
            "http://my-server:11434".to_string(),
        );
        assert_eq!(p.base_url, "http://my-server:11434");
        assert_eq!(p.model_id(), "codellama");
    }
}
