//! Google Gemini API provider.
//!
//! Implements the [`LlmProvider`] trait by calling the Google Generative Language API
//! (`POST /v1beta/models/{model}:generateContent` and `:streamGenerateContent`) via `reqwest`.

use std::pin::Pin;

use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures::{Stream, StreamExt};
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue};
use serde_json::Value;
use tracing::{debug, warn};

use openrustclaw_core::error::{Error, ProviderError, Result};
use openrustclaw_core::traits::LlmProvider;
use openrustclaw_core::types::{
    CompletionRequest, CompletionResponse, FinishReason, Message, Role, StreamChunk, TokenUsage,
    ToolCall, ToolFormat,
};

use crate::tool_formats::translate_tool_definition;

/// Default base URL for the Google Generative Language API.
const DEFAULT_BASE_URL: &str = "https://generativelanguage.googleapis.com";

/// LLM provider implementation for Google Gemini API.
pub struct GeminiProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl GeminiProvider {
    /// Create a new Gemini provider with the given API key and model.
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            model,
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }

    /// Create a new Gemini provider with a custom base URL.
    pub fn with_base_url(api_key: String, model: String, base_url: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            model,
            base_url,
        }
    }

    /// Get the maximum output tokens for the current model.
    fn model_max_output_tokens(&self) -> usize {
        match self.model.as_str() {
            "gemini-1.5-pro" | "gemini-1.5-pro-latest" => 8192,
            "gemini-1.5-flash" | "gemini-1.5-flash-latest" => 8192,
            "gemini-pro" | "gemini-1.0-pro" => 2048,
            _ => 4096, // Default fallback
        }
    }

    /// Build the request body for the Gemini API.
    fn build_request_body(&self, request: &CompletionRequest) -> Value {
        // Build the contents array (conversation history)
        let mut contents: Vec<Value> = Vec::new();

        // Add system prompt as a user message if present
        // Note: Gemini doesn't have a native system role, but some models support
        // system_instruction field. We'll include it in contents for compatibility.
        if let Some(ref system_prompt) = request.system_prompt {
            contents.push(serde_json::json!({
                "role": "user",
                "parts": [{"text": format!("System: {}", system_prompt)}],
            }));
            // Add a model response acknowledging the system prompt
            contents.push(serde_json::json!({
                "role": "model",
                "parts": [{"text": "Understood."}],
            }));
        }

        // Add conversation messages
        for msg in &request.messages {
            contents.push(self.message_to_gemini(msg));
        }

        let mut body = serde_json::json!({
            "contents": contents,
        });

        // Add generation config
        let mut generation_config = serde_json::json!({});

        if let Some(max_tokens) = request.max_tokens {
            generation_config["maxOutputTokens"] = serde_json::json!(max_tokens);
        }

        if let Some(temp) = request.temperature {
            generation_config["temperature"] = serde_json::json!(temp);
        }

        // Only add generationConfig if it has values
        if generation_config
            .as_object()
            .map(|o| !o.is_empty())
            .unwrap_or(false)
        {
            body["generationConfig"] = generation_config;
        }

        // Add tools if present
        if let Some(tools) = &request.tools {
            if !tools.is_empty() {
                let function_declarations: Vec<Value> =
                    tools.iter().map(|t| self.tool_to_gemini(t)).collect();
                body["tools"] = serde_json::json!([{
                    "function_declarations": function_declarations
                }]);
            }
        }

        body
    }

    /// Convert a unified [`Message`] to the Gemini content format.
    fn message_to_gemini(&self, msg: &Message) -> Value {
        let role = match msg.role {
            Role::User => "user",
            Role::Assistant => "model",
            Role::System => "user", // System messages are handled separately
            Role::Tool => "user",   // Tool results are sent as user messages
        };

        // Handle tool result messages
        if msg.role == Role::Tool {
            if let Some(ref tool_call_id) = msg.tool_call_id {
                return serde_json::json!({
                    "role": "user",
                    "parts": [{
                        "function_response": {
                            "name": tool_call_id, // Gemini uses function name, not call ID
                            "response": {
                                "result": msg.content
                            }
                        }
                    }]
                });
            }
        }

        // Handle assistant messages with tool calls
        if msg.role == Role::Assistant {
            if let Some(ref tool_calls) = msg.tool_calls {
                if !tool_calls.is_empty() {
                    let mut parts: Vec<Value> = Vec::new();

                    // Add text content if present
                    if !msg.content.is_empty() {
                        parts.push(serde_json::json!({
                            "text": msg.content,
                        }));
                    }

                    // Add function calls
                    for tc in tool_calls {
                        parts.push(serde_json::json!({
                            "function_call": {
                                "name": tc.name,
                                "args": tc.arguments,
                            }
                        }));
                    }

                    return serde_json::json!({
                        "role": "model",
                        "parts": parts,
                    });
                }
            }
        }

        // Standard text message
        serde_json::json!({
            "role": role,
            "parts": [{"text": msg.content}],
        })
    }

    /// Convert a unified [`ToolDefinition`] to Gemini's function declaration format.
    fn tool_to_gemini(&self, tool: &openrustclaw_core::types::ToolDefinition) -> Value {
        serde_json::json!({
            "name": tool.name,
            "description": tool.description,
            "parameters": tool.parameters,
        })
    }

    /// Parse the Gemini API response JSON into a [`CompletionResponse`].
    fn parse_response(&self, body: Value) -> Result<CompletionResponse> {
        // Check for prompt feedback (safety blocks, etc.)
        if let Some(feedback) = body.get("promptFeedback") {
            if let Some(block_reason) = feedback.get("blockReason") {
                return Err(Error::Provider(ProviderError::Request(format!(
                    "Prompt blocked: {}",
                    block_reason.as_str().unwrap_or("unknown")
                ))));
            }
        }

        // Get candidates array
        let candidates = body
            .get("candidates")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                Error::Provider(ProviderError::Parse(
                    "No candidates in Gemini response".to_string(),
                ))
            })?;

        if candidates.is_empty() {
            return Err(Error::Provider(ProviderError::Parse(
                "Empty candidates array in Gemini response".to_string(),
            )));
        }

        let candidate = &candidates[0];

        // Check for finish reason
        let finish_reason = candidate
            .get("finishReason")
            .and_then(|v| v.as_str())
            .unwrap_or("STOP");

        let finish_reason_enum = match finish_reason {
            "STOP" => FinishReason::Stop,
            "MAX_TOKENS" => FinishReason::MaxTokens,
            "SAFETY" | "RECITATION" => FinishReason::ContentFilter,
            "OTHER" => FinishReason::Stop,
            _ => FinishReason::Stop,
        };

        // Check for safety ratings that indicate content filtering
        if let Some(ratings) = candidate.get("safetyRatings").and_then(|v| v.as_array()) {
            let blocked = ratings
                .iter()
                .any(|r| r.get("blocked").and_then(|v| v.as_bool()).unwrap_or(false));
            if blocked {
                return Err(Error::Provider(ProviderError::Request(
                    "Content blocked by safety filters".to_string(),
                )));
            }
        }

        // Extract content
        let content = candidate.get("content").ok_or_else(|| {
            Error::Provider(ProviderError::Parse(
                "No content in Gemini candidate".to_string(),
            ))
        })?;

        let parts = content
            .get("parts")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                Error::Provider(ProviderError::Parse(
                    "No parts in Gemini content".to_string(),
                ))
            })?;

        // Extract text and tool calls from parts
        let mut text_content = String::new();
        let mut tool_calls: Vec<ToolCall> = Vec::new();

        for part in parts {
            if let Some(text) = part.get("text").and_then(|v| v.as_str()) {
                text_content.push_str(text);
            }

            if let Some(function_call) = part.get("functionCall") {
                let name = function_call
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let args = function_call
                    .get("args")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({}));

                tool_calls.push(ToolCall {
                    id: format!("{}_{}", name, tool_calls.len()), // Generate synthetic ID
                    name,
                    arguments: args,
                });
            }
        }

        // Build the message
        let mut message = Message::assistant(text_content);
        if !tool_calls.is_empty() {
            message.tool_calls = Some(tool_calls);
        }

        // Parse usage metadata
        let usage = if let Some(metadata) = body.get("usageMetadata") {
            TokenUsage {
                prompt_tokens: metadata
                    .get("promptTokenCount")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as usize,
                completion_tokens: metadata
                    .get("candidatesTokenCount")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as usize,
                total_tokens: metadata
                    .get("totalTokenCount")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as usize,
                cost_usd: None,
            }
        } else {
            TokenUsage::default()
        };

        // Generate a response ID
        let id = body
            .get("model")
            .and_then(|v| v.as_str())
            .map(|m| format!("gemini_{}", m))
            .unwrap_or_else(|| "gemini_response".to_string());

        Ok(CompletionResponse {
            id,
            message,
            model: self.model.clone(),
            usage,
            provider: "gemini".to_string(),
            finish_reason: finish_reason_enum,
        })
    }
}

#[async_trait]
impl LlmProvider for GeminiProvider {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let url = format!(
            "{}/v1beta/models/{}:generateContent?key={}",
            self.base_url, self.model, self.api_key
        );
        let body = self.build_request_body(&request);

        debug!(provider = "gemini", model = %self.model, "Sending completion request");

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                Error::Provider(ProviderError::Request(format!(
                    "Gemini request failed: {e}"
                )))
            })?;

        let status = response.status();

        // Handle rate limiting
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok());
            return Err(Error::Provider(ProviderError::RateLimited {
                provider: "gemini".to_string(),
                retry_after_secs: retry_after,
            }));
        }

        // Handle authentication errors
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            let error_body = response.text().await.unwrap_or_default();
            return Err(Error::Provider(ProviderError::AuthFailed {
                provider: "gemini".to_string(),
                message: error_body,
            }));
        }

        // Handle model not found
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::Provider(ProviderError::ModelNotFound {
                provider: "gemini".to_string(),
                model: self.model.clone(),
            }));
        }

        // Handle server errors
        if status.is_server_error() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(Error::Provider(ProviderError::Unavailable {
                provider: "gemini".to_string(),
                message: format!("Server error {status}: {error_body}"),
            }));
        }

        // Handle context length exceeded (429 with specific error or 400)
        if status == reqwest::StatusCode::BAD_REQUEST {
            let error_body = response.text().await.unwrap_or_default();
            let error_json: Value = serde_json::from_str(&error_body).unwrap_or_default();

            // Check for specific error codes
            if let Some(error) = error_json.get("error") {
                let code = error.get("code").and_then(|v| v.as_i64());
                let message = error
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&error_body);

                // 400 = context length exceeded or bad request
                if message.to_lowercase().contains("token")
                    || message.to_lowercase().contains("context")
                {
                    return Err(Error::Provider(ProviderError::ContextLengthExceeded {
                        used: 0,
                        max: self.max_tokens(),
                    }));
                }
            }

            warn!(
                provider = "gemini",
                status = %status,
                body = %error_body,
                "Bad request error"
            );
            return Err(Error::Provider(ProviderError::Request(format!(
                "Gemini API error {status}: {error_body}"
            ))));
        }

        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            warn!(
                provider = "gemini",
                status = %status,
                body = %error_body,
                "Unexpected error response"
            );
            return Err(Error::Provider(ProviderError::Request(format!(
                "Gemini API error {status}: {error_body}"
            ))));
        }

        let response_body: Value = response.json().await.map_err(|e| {
            Error::Provider(ProviderError::Parse(format!(
                "Failed to parse Gemini response: {e}"
            )))
        })?;

        self.parse_response(response_body)
    }

    async fn stream(
        &self,
        request: CompletionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>> {
        let url = format!(
            "{}/v1beta/models/{}:streamGenerateContent?key={}&alt=sse",
            self.base_url, self.model, self.api_key
        );
        let body = self.build_request_body(&request);

        debug!(provider = "gemini", model = %self.model, "Sending streaming completion request");

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                Error::Provider(ProviderError::Request(format!(
                    "Gemini streaming request failed: {e}"
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
                provider: "gemini".to_string(),
                retry_after_secs: retry_after,
            }));
        }

        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            let error_body = response.text().await.unwrap_or_default();
            return Err(Error::Provider(ProviderError::AuthFailed {
                provider: "gemini".to_string(),
                message: error_body,
            }));
        }

        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::Provider(ProviderError::ModelNotFound {
                provider: "gemini".to_string(),
                model: self.model.clone(),
            }));
        }

        if status.is_server_error() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(Error::Provider(ProviderError::Unavailable {
                provider: "gemini".to_string(),
                message: format!("Server error {status}: {error_body}"),
            }));
        }

        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            warn!(
                provider = "gemini",
                status = %status,
                body = %error_body,
                "Unexpected error response"
            );
            return Err(Error::Provider(ProviderError::Request(format!(
                "Gemini API error {status}: {error_body}"
            ))));
        }

        let model = self.model.clone();
        let provider_name = "gemini".to_string();

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
                            let data: Value = match serde_json::from_str(&event.data) {
                                Ok(v) => v,
                                Err(e) => {
                                    return Err(Error::Provider(ProviderError::StreamError {
                                        provider: "gemini".to_string(),
                                        message: format!("Failed to parse SSE data: {e}"),
                                    }));
                                }
                            };

                            // Check for prompt feedback (blocking)
                            if let Some(feedback) = data.get("promptFeedback") {
                                if feedback.get("blockReason").is_some() {
                                    return Ok(StreamChunk::Done {
                                        response: CompletionResponse {
                                            id: "streamed".to_string(),
                                            message: Message::assistant(""),
                                            model: model.clone(),
                                            usage: TokenUsage::default(),
                                            provider: provider_name.clone(),
                                            finish_reason: FinishReason::ContentFilter,
                                        },
                                    });
                                }
                            }

                            // Get candidates
                            let candidates = match data.get("candidates").and_then(|v| v.as_array())
                            {
                                Some(c) if !c.is_empty() => c,
                                _ => {
                                    // Check for usage metadata (end of stream)
                                    if data.get("usageMetadata").is_some() {
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
                                    return Ok(StreamChunk::ContentDelta {
                                        delta: String::new(),
                                    });
                                }
                            };

                            let candidate = &candidates[0];

                            // Check finish reason
                            if let Some(finish_reason) =
                                candidate.get("finishReason").and_then(|v| v.as_str())
                            {
                                if finish_reason != "FINISH_REASON_UNSPECIFIED"
                                    && finish_reason != "STOP"
                                {
                                    let reason = match finish_reason {
                                        "MAX_TOKENS" => FinishReason::MaxTokens,
                                        "SAFETY" | "RECITATION" => FinishReason::ContentFilter,
                                        _ => FinishReason::Stop,
                                    };
                                    return Ok(StreamChunk::Done {
                                        response: CompletionResponse {
                                            id: "streamed".to_string(),
                                            message: Message::assistant(""),
                                            model: model.clone(),
                                            usage: TokenUsage::default(),
                                            provider: provider_name.clone(),
                                            finish_reason: reason,
                                        },
                                    });
                                }
                            }

                            // Extract content delta
                            if let Some(content) = candidate.get("content") {
                                if let Some(parts) = content.get("parts").and_then(|v| v.as_array())
                                {
                                    for part in parts {
                                        // Text content
                                        if let Some(text) =
                                            part.get("text").and_then(|v| v.as_str())
                                        {
                                            return Ok(StreamChunk::ContentDelta {
                                                delta: text.to_string(),
                                            });
                                        }

                                        // Function call (tool call)
                                        if let Some(function_call) = part.get("functionCall") {
                                            let name = function_call
                                                .get("name")
                                                .and_then(|v| v.as_str())
                                                .map(|s| s.to_string());
                                            let args = function_call
                                                .get("args")
                                                .cloned()
                                                .unwrap_or_else(|| serde_json::json!({}));

                                            return Ok(StreamChunk::ToolCallDelta {
                                                id: name.clone().unwrap_or_default(),
                                                name,
                                                arguments_delta: args.to_string(),
                                            });
                                        }
                                    }
                                }
                            }

                            Ok(StreamChunk::ContentDelta {
                                delta: String::new(),
                            })
                        }
                        Err(e) => Err(Error::Provider(ProviderError::StreamError {
                            provider: "gemini".to_string(),
                            message: format!("SSE stream error: {e}"),
                        })),
                    }
                },
            )
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
        match self.model.as_str() {
            "gemini-1.5-pro" | "gemini-1.5-pro-latest" => 1_048_576, // 1M tokens
            "gemini-1.5-flash" | "gemini-1.5-flash-latest" => 1_048_576, // 1M tokens
            "gemini-pro" | "gemini-1.0-pro" => 32_768,
            _ => 1_048_576, // Default to 1M for newer models
        }
    }

    fn provider_name(&self) -> &str {
        "gemini"
    }

    fn supports_strict_tools(&self) -> bool {
        true
    }

    fn supports_streaming_tool_deltas(&self) -> bool {
        false // Gemini streams tool calls but not as fine-grained deltas
    }

    fn native_tool_format(&self) -> ToolFormat {
        // Gemini's tool format is similar to OpenAI's function calling
        ToolFormat::OpenAi
    }
}

/// Create a new Gemini provider with the given API key and model.
///
/// This is a convenience function for creating a Gemini provider.
pub fn create_gemini_provider(api_key: String, model: String) -> GeminiProvider {
    GeminiProvider::new(api_key, model)
}

#[cfg(test)]
mod tests {
    use super::*;
    use openrustclaw_core::types::ToolDefinition;

    fn make_provider() -> GeminiProvider {
        GeminiProvider::new("test-key".to_string(), "gemini-1.5-flash".to_string())
    }

    #[test]
    fn provider_metadata() {
        let p = make_provider();
        assert_eq!(p.provider_name(), "gemini");
        assert_eq!(p.model_id(), "gemini-1.5-flash");
        assert_eq!(p.max_tokens(), 1_048_576);
        assert!(p.supports_strict_tools());
        assert!(!p.supports_streaming_tool_deltas());
        assert_eq!(p.native_tool_format(), ToolFormat::OpenAi);
    }

    #[test]
    fn max_tokens_by_model() {
        let pro = GeminiProvider::new("key".to_string(), "gemini-1.5-pro".to_string());
        assert_eq!(pro.max_tokens(), 1_048_576);

        let flash = GeminiProvider::new("key".to_string(), "gemini-1.5-flash".to_string());
        assert_eq!(flash.max_tokens(), 1_048_576);

        let legacy = GeminiProvider::new("key".to_string(), "gemini-pro".to_string());
        assert_eq!(legacy.max_tokens(), 32_768);
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
            system_prompt: Some("Be helpful.".to_string()),
            stream: false,
        };
        let body = p.build_request_body(&request);

        // System prompt should create a user message
        let contents = body["contents"].as_array().unwrap();
        assert!(contents.len() >= 2); // System message + model response + user message
        assert_eq!(contents[0]["role"], "user");
        assert!(
            contents[0]["parts"][0]["text"]
                .as_str()
                .unwrap()
                .contains("System:")
        );

        // Generation config
        assert_eq!(body["generationConfig"]["maxOutputTokens"], 1024);
        let temp = body["generationConfig"]["temperature"].as_f64().unwrap();
        assert!((temp - 0.7).abs() < 0.001);
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
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "location": {"type": "string"}
                    }
                }),
                strict: true,
            }]),
            system_prompt: None,
            stream: false,
        };
        let body = p.build_request_body(&request);
        let tools = body["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 1);
        let declarations = tools[0]["function_declarations"].as_array().unwrap();
        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0]["name"], "get_weather");
        assert_eq!(declarations[0]["parameters"]["type"], "object");
    }

    #[test]
    fn parse_text_response() {
        let p = make_provider();
        let response_json = serde_json::json!({
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [{"text": "Hello! How can I help?"}]
                },
                "finishReason": "STOP",
                "index": 0,
                "safetyRatings": [
                    {"category": "HARM_CATEGORY_HARASSMENT", "probability": "NEGLIGIBLE"}
                ]
            }],
            "usageMetadata": {
                "promptTokenCount": 10,
                "candidatesTokenCount": 8,
                "totalTokenCount": 18
            },
            "model": "gemini-1.5-flash"
        });
        let response = p.parse_response(response_json).unwrap();
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
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [
                        {"text": "Let me check the weather."},
                        {
                            "functionCall": {
                                "name": "get_weather",
                                "args": {"location": "Paris"}
                            }
                        }
                    ]
                },
                "finishReason": "STOP",
                "index": 0
            }],
            "usageMetadata": {
                "promptTokenCount": 20,
                "candidatesTokenCount": 30,
                "totalTokenCount": 50
            }
        });
        let response = p.parse_response(response_json).unwrap();
        assert_eq!(response.message.content, "Let me check the weather.");
        assert_eq!(response.finish_reason, FinishReason::Stop);
        let calls = response.message.tool_calls.unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "get_weather");
        assert_eq!(calls[0].arguments["location"], "Paris");
    }

    #[test]
    fn parse_safety_blocked_response() {
        let p = make_provider();
        let response_json = serde_json::json!({
            "promptFeedback": {
                "blockReason": "SAFETY",
                "safetyRatings": [
                    {"category": "HARM_CATEGORY_HARASSMENT", "probability": "HIGH"}
                ]
            }
        });
        let result = p.parse_response(response_json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("blocked"));
    }

    #[test]
    fn parse_max_tokens_response() {
        let p = make_provider();
        let response_json = serde_json::json!({
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [{"text": "This is a partial response..."}]
                },
                "finishReason": "MAX_TOKENS",
                "index": 0
            }],
            "usageMetadata": {
                "promptTokenCount": 100,
                "candidatesTokenCount": 1024,
                "totalTokenCount": 1124
            }
        });
        let response = p.parse_response(response_json).unwrap();
        assert_eq!(response.finish_reason, FinishReason::MaxTokens);
    }

    #[test]
    fn message_to_gemini_user() {
        let p = make_provider();
        let msg = Message::user("Hello");
        let result = p.message_to_gemini(&msg);
        assert_eq!(result["role"], "user");
        assert_eq!(result["parts"][0]["text"], "Hello");
    }

    #[test]
    fn message_to_gemini_assistant() {
        let p = make_provider();
        let msg = Message::assistant("Hi there");
        let result = p.message_to_gemini(&msg);
        assert_eq!(result["role"], "model");
        assert_eq!(result["parts"][0]["text"], "Hi there");
    }

    #[test]
    fn message_to_gemini_tool_result() {
        let p = make_provider();
        let msg = Message::tool("get_weather_123", "The weather is sunny.");
        let result = p.message_to_gemini(&msg);
        assert_eq!(result["role"], "user");
        assert_eq!(
            result["parts"][0]["function_response"]["name"],
            "get_weather_123"
        );
        assert_eq!(
            result["parts"][0]["function_response"]["response"]["result"],
            "The weather is sunny."
        );
    }

    #[test]
    fn message_to_gemini_with_tool_calls() {
        let p = make_provider();
        let mut msg = Message::assistant("I'll check");
        msg.tool_calls = Some(vec![ToolCall {
            id: "call_123".to_string(),
            name: "get_weather".to_string(),
            arguments: serde_json::json!({"location": "Tokyo"}),
        }]);
        let result = p.message_to_gemini(&msg);
        assert_eq!(result["role"], "model");
        let parts = result["parts"].as_array().unwrap();
        assert_eq!(parts[0]["text"], "I'll check");
        assert_eq!(parts[1]["function_call"]["name"], "get_weather");
        assert_eq!(parts[1]["function_call"]["args"]["location"], "Tokyo");
    }

    #[test]
    fn tool_to_gemini_format() {
        let p = make_provider();
        let tool = ToolDefinition {
            name: "get_weather".to_string(),
            description: "Get the weather".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "location": {"type": "string"}
                },
                "required": ["location"]
            }),
            strict: false,
        };
        let result = p.tool_to_gemini(&tool);
        assert_eq!(result["name"], "get_weather");
        assert_eq!(result["description"], "Get the weather");
        assert_eq!(result["parameters"]["type"], "object");
    }

    #[test]
    fn create_gemini_provider_helper() {
        let provider =
            create_gemini_provider("my-api-key".to_string(), "gemini-1.5-pro".to_string());
        assert_eq!(provider.model_id(), "gemini-1.5-pro");
        assert_eq!(provider.provider_name(), "gemini");
    }
}
