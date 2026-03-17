//! Type definitions for the Anthropic Messages API.

mod message;
mod request;
mod response;
mod tool;

pub use message::{ContentBlock, ImageContent, ImageSource, Message, MessageRole, TextBlock};
pub use request::{MessageRequest, MessageRequestBuilder, Metadata, ToolChoice};
pub use response::{MessageResponse, MessageType, StopReason, Usage};
pub use tool::{Tool, ToolInputSchema, ToolResult, ToolUse};

#[cfg(feature = "streaming")]
pub use message::{ContentBlockDelta, TextDelta};

use serde::{Deserialize, Serialize};

/// A cache control directive for prompt caching (beta feature).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CacheControl {
    /// Cache this content for later use.
    Ephemeral,
}

/// System prompt content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SystemContent {
    /// Simple text system prompt.
    Text(String),
    /// Structured system prompt with cache control.
    Blocks(Vec<SystemBlock>),
}

/// A system block with optional cache control.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemBlock {
    /// The type of content.
    #[serde(rename = "type")]
    pub block_type: String,
    /// The text content.
    pub text: String,
    /// Cache control directive.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControl>,
}

impl SystemBlock {
    /// Create a new text system block.
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            block_type: "text".to_string(),
            text: content.into(),
            cache_control: None,
        }
    }

    /// Create a new text system block with cache control.
    pub fn text_ephemeral(content: impl Into<String>) -> Self {
        Self {
            block_type: "text".to_string(),
            text: content.into(),
            cache_control: Some(CacheControl::Ephemeral),
        }
    }
}

/// Thinking configuration (beta feature for extended thinking).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingConfig {
    /// The type of thinking to enable.
    #[serde(rename = "type")]
    pub thinking_type: String,
    /// Budget tokens for thinking.
    pub budget_tokens: usize,
}

impl ThinkingConfig {
    /// Create a new thinking configuration.
    pub fn new(budget_tokens: usize) -> Self {
        Self {
            thinking_type: "enabled".to_string(),
            budget_tokens,
        }
    }
}
