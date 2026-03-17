//! Type definitions for the Azure OpenAI API.

mod chat;
mod embeddings;
mod shared;

pub use chat::{
    ChatChoice, ChatMessage, ChatResponse, Function, FunctionCall, Role, Tool, ToolCall,
    ToolChoice, ToolChoiceFunction, ToolType,
};
pub use embeddings::{Embedding, EmbeddingUsage, EmbeddingsResponse};
pub use shared::{FinishReason, TokenUsage};

/// Content filter result details.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ContentFilterDetail {
    /// Whether the content was filtered.
    pub filtered: bool,
    /// The severity level.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<String>,
}

/// Content filter results for a prompt or response.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ContentFilterResults {
    /// Hate speech filter result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hate: Option<ContentFilterDetail>,
    /// Self-harm filter result.
    #[serde(rename = "self_harm", skip_serializing_if = "Option::is_none")]
    pub self_harm: Option<ContentFilterDetail>,
    /// Sexual content filter result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sexual: Option<ContentFilterDetail>,
    /// Violence filter result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub violence: Option<ContentFilterDetail>,
    /// Profanity filter result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profanity: Option<ContentFilterDetail>,
    /// Custom blocklist filter result.
    #[serde(rename = "custom_blocklists", skip_serializing_if = "Option::is_none")]
    pub custom_blocklists: Option<ContentFilterDetail>,
    /// Error if content filter failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<serde_json::Value>,
}

/// Content filter results for prompt.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct PromptFilterResult {
    /// The index of the prompt.
    pub prompt_index: usize,
    /// The content filter results.
    #[serde(rename = "content_filter_results")]
    pub content_filter: Option<ContentFilterResults>,
}

/// Error information.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ApiError {
    /// The error message.
    pub message: String,
    /// The error type.
    #[serde(rename = "type")]
    pub error_type: String,
    /// The parameter that caused the error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub param: Option<String>,
    /// The error code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

/// API error response.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ApiErrorResponse {
    /// The error details.
    pub error: ApiError,
}
