//! Streaming response handling for OpenRouter.

use serde::{Deserialize, Serialize};

/// A chat completion chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionChunk {
    /// Unique identifier.
    pub id: String,
    /// Object type.
    pub object: String,
    /// Unix timestamp.
    pub created: i64,
    /// Model used.
    pub model: String,
    /// Choices.
    pub choices: Vec<StreamChoice>,
}

/// A choice in a streaming chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChoice {
    /// Index.
    pub index: usize,
    /// Delta.
    pub delta: StreamDelta,
    /// Finish reason.
    pub finish_reason: Option<String>,
}

/// A delta in a stream.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StreamDelta {
    /// Role.
    pub role: Option<crate::types::Role>,
    /// Content.
    pub content: Option<String>,
    /// Tool calls.
    pub tool_calls: Option<Vec<crate::types::ToolCall>>,
}
