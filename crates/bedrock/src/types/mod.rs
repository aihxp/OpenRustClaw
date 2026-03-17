//! Type definitions for AWS Bedrock.

mod content;
mod conversation;
mod inference;
mod tool;

pub use content::{
    ContentBlock, ContentBlockDelta, GuardrailAssessment, GuardrailContentFilter,
    GuardrailContentPolicyAction, GuardrailContentPolicyConfig, GuardrailCustomWord,
    GuardrailManagedWord, GuardrailManagedWordType, GuardrailPiiEntity, GuardrailPiiEntityConfig,
    GuardrailRegex, GuardrailRegexConfig, GuardrailSensitiveInformationPolicyAction,
    GuardrailSensitiveInformationPolicyConfig, GuardrailTopic, GuardrailTopicPolicyAction,
    GuardrailTopicPolicyConfig, GuardrailWordAction, GuardrailWordConfig,
    GuardrailWordPolicyAction, GuardrailWordPolicyConfig, ImageBlock, ImageFormat, ImageSource,
    S3Location, VideoBlock, VideoFormat, VideoSource,
};

// Re-export GuardrailTrace from converse module
pub use crate::converse::{ConverseTrace as GuardrailTrace, GuardrailTrace as GuardrailTraceInner};

pub use conversation::{ConversationRole, InlineConversation, Message, StopReason, TokenUsage};

pub use inference::{
    GuardrailConfiguration, InferenceConfiguration, PerformanceConfiguration, PromptVariable,
    RequestMetadata, SystemContentBlock, Trace,
};

pub use content::ToolResultContent;
pub use tool::{
    AnyToolChoice, AutoToolChoice, SpecificToolChoice, Tool, ToolChoice, ToolInputSchema,
    ToolResultBlock, ToolSpecification, ToolUseBlock,
};

use serde::{Deserialize, Serialize};

/// A unique identifier for a conversation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConversationId(pub String);

impl ConversationId {
    /// Create a new conversation ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Generate a random conversation ID.
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

impl std::fmt::Display for ConversationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for ConversationId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for ConversationId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Additional model request fields (provider-specific).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AdditionalModelRequestFields {
    /// Raw JSON fields for provider-specific options.
    #[serde(flatten)]
    pub fields: serde_json::Map<String, serde_json::Value>,
}

impl AdditionalModelRequestFields {
    /// Create empty additional fields.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a field.
    pub fn with_field(
        mut self,
        key: impl Into<String>,
        value: impl Into<serde_json::Value>,
    ) -> Self {
        self.fields.insert(key.into(), value.into());
        self
    }
}

/// Additional model result fields (provider-specific).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AdditionalModelResultFields {
    /// Raw JSON fields for provider-specific output.
    #[serde(flatten)]
    pub fields: serde_json::Map<String, serde_json::Value>,
}
