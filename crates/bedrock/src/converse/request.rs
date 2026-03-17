//! Converse API request types.

use serde::{Deserialize, Serialize};

use crate::constants::endpoints;
use crate::types::{
    AdditionalModelRequestFields, GuardrailConfiguration, InferenceConfiguration, Message,
    PerformanceConfiguration, RequestMetadata, SystemContentBlock,
};

/// A request to the Converse API.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConverseRequest {
    /// The model ID to use.
    pub model_id: String,
    /// The conversation messages.
    pub messages: Vec<Message>,
    /// System prompt content blocks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<Vec<SystemContentBlock>>,
    /// Inference configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inference_config: Option<InferenceConfiguration>,
    /// Tool configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_config: Option<super::ToolConfig>,
    /// Guardrail configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guardrail_config: Option<GuardrailConfiguration>,
    /// Additional model-specific request fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_model_request_fields: Option<AdditionalModelRequestFields>,
    /// Additional response field paths to include.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_model_response_field_paths: Option<Vec<String>>,
    /// Request metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_metadata: Option<RequestMetadata>,
    /// Performance configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub performance_config: Option<PerformanceConfiguration>,
}

impl ConverseRequest {
    /// Create a new ConverseRequest builder.
    pub fn builder(model_id: impl Into<String>) -> super::ConverseRequestBuilder {
        let mut builder = super::ConverseRequestBuilder::new();
        builder.model_id = Some(model_id.into());
        builder
    }

    /// Get the endpoint path for this request.
    pub fn endpoint_path(&self) -> String {
        endpoints::CONVERSE.replace("{modelId}", &self.model_id)
    }

    /// Get the streaming endpoint path.
    pub fn stream_endpoint_path(&self) -> String {
        endpoints::CONVERSE_STREAM.replace("{modelId}", &self.model_id)
    }

    /// Add a message to this request.
    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    /// Set the system prompt.
    pub fn set_system(&mut self, system: impl Into<String>) {
        self.system = Some(vec![SystemContentBlock::text(system)]);
    }

    /// Get the conversation ID if present in metadata.
    pub fn conversation_id(&self) -> Option<&str> {
        self.request_metadata
            .as_ref()
            .and_then(|m| m.trace_id.as_deref())
    }
}

/// Request for ConverseStream API.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct ConverseStreamRequest {
    /// The base Converse request.
    #[serde(flatten)]
    pub base: ConverseRequest,
}

impl ConverseStreamRequest {
    /// Create from a ConverseRequest.
    #[allow(dead_code)]
    pub fn from_request(request: ConverseRequest) -> Self {
        Self { base: request }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ConversationRole, ContentBlock};

    #[test]
    fn test_converse_request() {
        let request = ConverseRequest {
            model_id: "anthropic.claude-3-sonnet-20240229-v1:0".to_string(),
            messages: vec![Message::new(
                ConversationRole::User,
                vec![ContentBlock::text("Hello")],
            )],
            system: None,
            inference_config: None,
            tool_config: None,
            guardrail_config: None,
            additional_model_request_fields: None,
            additional_model_response_field_paths: None,
            request_metadata: None,
            performance_config: None,
        };

        assert_eq!(request.endpoint_path(), "/model/anthropic.claude-3-sonnet-20240229-v1:0/converse");
        assert_eq!(request.stream_endpoint_path(), "/model/anthropic.claude-3-sonnet-20240229-v1:0/converse-stream");
    }
}
