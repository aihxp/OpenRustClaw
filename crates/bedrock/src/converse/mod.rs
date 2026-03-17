//! AWS Bedrock Converse API.
//!
//! The Converse API provides a unified interface for all Bedrock models.
//! It supports multi-turn conversations, tool use, vision, and streaming.

use std::sync::Arc;

use tracing::{debug, trace};

use crate::auth::{service_endpoint, Service, SigV4Signer};

use crate::error::{BedrockError, Result};
use crate::types::{
    AdditionalModelRequestFields, ConversationId, GuardrailConfiguration, InferenceConfiguration,
    InlineConversation, Message, PerformanceConfiguration, RequestMetadata, SystemContentBlock,
    Tool, ToolChoice,
};

#[cfg(feature = "streaming")]
use crate::streaming::{StreamEvent, StreamResult};

mod request;
mod response;

pub use request::ConverseRequest;
pub use response::{
    ConverseResponse, ConverseStreamResponse, ConverseTrace, ContentBlockStart, GuardrailTrace,
    Message as ConverseMessage, Metrics, Output, StreamCollector, StreamEvent as ConverseStreamEvent,
    StreamMessageStart, StreamMetadata,
};

/// Client for the Converse API.
#[derive(Debug, Clone)]
pub struct ConverseClient {
    inner: Arc<ConverseClientInner>,
}

#[derive(Debug)]
struct ConverseClientInner {
    http: reqwest::Client,
    _region: crate::auth::Region,
    signer: SigV4Signer,
    endpoint: String,
}

impl ConverseClient {
    /// Create a new Converse API client.
    pub fn new(
        http: reqwest::Client,
        region: crate::auth::Region,
        signer: SigV4Signer,
    ) -> Self {
        let endpoint = service_endpoint(Service::BedrockRuntime, &region);

        Self {
            inner: Arc::new(ConverseClientInner {
                http,
                _region: region,
                signer,
                endpoint,
            }),
        }
    }

    /// Invoke the Converse API for a single response.
    pub async fn invoke(&self, request: ConverseRequest) -> Result<ConverseResponse> {
        trace!(model_id = %request.model_id, "Invoking Converse API");

        let url = format!("{}{}", self.inner.endpoint, request.endpoint_path());
        let body = serde_json::to_value(&request)?;

        debug!(url = %url, "Sending Converse request");

        // Sign the request
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/json"),
        );

        let body_bytes = body.to_string().into_bytes();
        self.inner.signer.sign_request(
            "POST",
            &url,
            &mut headers,
            &body_bytes,
            Service::BedrockRuntime,
        )?;

        // Send the request
        let response = self
            .inner
            .http
            .post(&url)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .map_err(BedrockError::from)?;

        if !response.status().is_success() {
            return Err(BedrockError::from_response(response).await);
        }

        let response_body: ConverseResponse = response.json().await.map_err(|e| {
            BedrockError::Internal {
                message: format!("Failed to parse response: {e}"),
            }
        })?;

        debug!("Converse request completed successfully");

        Ok(response_body)
    }

    /// Invoke the Converse API with streaming response.
    #[cfg(feature = "streaming")]
    pub async fn stream(
        &self,
        request: ConverseRequest,
    ) -> Result<impl futures::Stream<Item = StreamResult<StreamEvent>>> {
        use eventsource_stream::Eventsource;
        use futures::StreamExt;

        trace!(model_id = %request.model_id, "Starting Converse stream");

        let url = format!("{}{}", self.inner.endpoint, request.stream_endpoint_path());
        let body = serde_json::to_value(&request)?;

        debug!(url = %url, "Sending Converse stream request");

        // Sign the request
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        headers.insert(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static("text/event-stream"),
        );

        let body_bytes = body.to_string().into_bytes();
        self.inner.signer.sign_request(
            "POST",
            &url,
            &mut headers,
            &body_bytes,
            Service::BedrockRuntime,
        )?;

        // Send the request
        let response = self
            .inner
            .http
            .post(&url)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .map_err(BedrockError::from)?;

        if !response.status().is_success() {
            return Err(BedrockError::from_response(response).await);
        }

        let stream = response
            .bytes_stream()
            .eventsource()
            .map(|event| {
                match event {
                    Ok(event) => {
                        trace!(event_type = %event.event, "Received stream event");
                        
                        // Handle different event types
                        match event.event.as_str() {
                            "messageStart" | "contentBlockStart" | "contentBlockDelta" 
                            | "contentBlockStop" | "messageStop" | "metadata" => {
                                match serde_json::from_str::<StreamEvent>(&event.data) {
                                    Ok(stream_event) => Ok(stream_event),
                                    Err(e) => Err(BedrockError::Stream {
                                        message: format!("Failed to parse stream event: {e}"),
                                    }),
                                }
                            }
                            _ => {
                                // Unknown event type, try to parse anyway
                                match serde_json::from_str::<StreamEvent>(&event.data) {
                                    Ok(stream_event) => Ok(stream_event),
                                    Err(_) => Err(BedrockError::Stream {
                                        message: format!("Unknown event type: {}", event.event),
                                    }),
                                }
                            }
                        }
                    }
                    Err(e) => Err(BedrockError::Stream {
                        message: format!("Stream error: {e}"),
                    }),
                }
            });

        Ok(stream)
    }

    /// Invoke the Converse API with a conversation.
    pub async fn converse(&self, request: ConverseRequest) -> Result<ConverseResponse> {
        self.invoke(request).await
    }

    /// Continue a conversation with a new message.
    pub async fn continue_conversation(
        &self,
        _conversation_id: &ConversationId,
        message: Message,
        model_id: impl Into<String>,
    ) -> Result<ConverseResponse> {
        // In a real implementation, this would fetch the conversation history
        // from a store using the conversation_id, add the new message, and invoke.
        // For now, we create a new request with just the new message.
        let request = ConverseRequest::builder(model_id)
            .message(message)
            .build();

        self.invoke(request).await
    }
}

/// Builder for Converse API requests.
#[derive(Debug, Default)]
pub struct ConverseRequestBuilder {
    model_id: Option<String>,
    messages: Vec<Message>,
    system: Option<Vec<SystemContentBlock>>,
    inference_config: Option<InferenceConfiguration>,
    tool_config: Option<ToolConfig>,
    guardrail_config: Option<GuardrailConfiguration>,
    additional_model_request_fields: Option<AdditionalModelRequestFields>,
    additional_model_response_field_paths: Option<Vec<String>>,
    request_metadata: Option<RequestMetadata>,
    performance_config: Option<PerformanceConfiguration>,
}

/// Tool configuration for Converse API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    tools: Vec<Tool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<ToolChoice>,
}

impl ConverseRequestBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the model ID.
    pub fn model(mut self, model_id: impl Into<String>) -> Self {
        self.model_id = Some(model_id.into());
        self
    }

    /// Add a message.
    pub fn message(mut self, message: Message) -> Self {
        self.messages.push(message);
        self
    }

    /// Add a text message with the given role.
    pub fn text_message(mut self, role: crate::types::ConversationRole, text: impl Into<String>) -> Self {
        self.messages.push(Message::text(role, text));
        self
    }

    /// Add a user message.
    pub fn user_message(mut self, text: impl Into<String>) -> Self {
        self.messages.push(Message::user_text(text));
        self
    }

    /// Add an assistant message.
    pub fn assistant_message(mut self, text: impl Into<String>) -> Self {
        self.messages.push(Message::assistant_text(text));
        self
    }

    /// Set the conversation messages.
    pub fn messages(mut self, messages: Vec<Message>) -> Self {
        self.messages = messages;
        self
    }

    /// Use a conversation.
    pub fn conversation(mut self, conversation: InlineConversation) -> Self {
        self.messages = conversation.messages;
        self
    }

    /// Set the system prompt.
    pub fn system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(vec![SystemContentBlock::text(system)]);
        self
    }

    /// Set multiple system content blocks.
    pub fn system_blocks(mut self, blocks: Vec<SystemContentBlock>) -> Self {
        self.system = Some(blocks);
        self
    }

    /// Set inference configuration.
    pub fn inference_config(mut self, config: InferenceConfiguration) -> Self {
        self.inference_config = Some(config);
        self
    }

    /// Set maximum tokens.
    pub fn max_tokens(mut self, max_tokens: i32) -> Self {
        self.inference_config
            .get_or_insert_with(InferenceConfiguration::default)
            .max_tokens = Some(max_tokens);
        self
    }

    /// Set temperature.
    pub fn temperature(mut self, temperature: f32) -> Self {
        self.inference_config
            .get_or_insert_with(InferenceConfiguration::default)
            .temperature = Some(temperature.clamp(0.0, 1.0));
        self
    }

    /// Set top-p.
    pub fn top_p(mut self, top_p: f32) -> Self {
        self.inference_config
            .get_or_insert_with(InferenceConfiguration::default)
            .top_p = Some(top_p.clamp(0.0, 1.0));
        self
    }

    /// Add a tool.
    pub fn tool(mut self, tool: Tool) -> Self {
        self.tool_config
            .get_or_insert_with(|| ToolConfig {
                tools: Vec::new(),
                tool_choice: None,
            })
            .tools
            .push(tool);
        self
    }

    /// Set tools.
    pub fn tools(mut self, tools: Vec<Tool>) -> Self {
        self.tool_config = Some(ToolConfig {
            tools,
            tool_choice: None,
        });
        self
    }

    /// Set tool choice.
    pub fn tool_choice(mut self, choice: ToolChoice) -> Self {
        self.tool_config
            .get_or_insert_with(|| ToolConfig {
                tools: Vec::new(),
                tool_choice: None,
            })
            .tool_choice = Some(choice);
        self
    }

    /// Set guardrail configuration.
    pub fn guardrail(mut self, config: GuardrailConfiguration) -> Self {
        self.guardrail_config = Some(config);
        self
    }

    /// Set additional model request fields.
    pub fn additional_fields(mut self, fields: AdditionalModelRequestFields) -> Self {
        self.additional_model_request_fields = Some(fields);
        self
    }

    /// Set performance configuration.
    pub fn performance(mut self, config: PerformanceConfiguration) -> Self {
        self.performance_config = Some(config);
        self
    }

    /// Build the request.
    pub fn build(self) -> ConverseRequest {
        ConverseRequest {
            model_id: self.model_id.expect("model_id is required"),
            messages: self.messages,
            system: self.system,
            inference_config: self.inference_config,
            tool_config: self.tool_config,
            guardrail_config: self.guardrail_config,
            additional_model_request_fields: self.additional_model_request_fields,
            additional_model_response_field_paths: self.additional_model_response_field_paths,
            request_metadata: self.request_metadata,
            performance_config: self.performance_config,
        }
    }
}

use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ConversationRole;

    #[test]
    fn test_converse_request_builder() {
        let request = ConverseRequest::builder("anthropic.claude-3-sonnet-20240229-v1:0")
            .user_message("Hello")
            .assistant_message("Hi there!")
            .max_tokens(1000)
            .temperature(0.7)
            .build();

        assert_eq!(request.model_id, "anthropic.claude-3-sonnet-20240229-v1:0");
        assert_eq!(request.messages.len(), 2);
        assert_eq!(request.inference_config.unwrap().max_tokens, Some(1000));
    }

    #[test]
    fn test_converse_request_builder_with_tools() {
        let tool = Tool::new("calculator", "A calculator tool")
            .with_property("expression", "string", "Math expression");

        let request = ConverseRequest::builder("anthropic.claude-3-sonnet-20240229-v1:0")
            .user_message("Calculate 2+2")
            .tool(tool)
            .tool_choice(ToolChoice::auto())
            .build();

        assert!(request.tool_config.is_some());
        assert_eq!(request.tool_config.unwrap().tools.len(), 1);
    }
}
