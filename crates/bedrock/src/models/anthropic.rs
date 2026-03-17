//! Anthropic Claude model-specific types.
//!
//! For most use cases, use the Converse API which provides a unified interface.
//! These types are for direct InvokeModel access when needed.

use serde::{Deserialize, Serialize};

/// Anthropic Claude request body for InvokeModel.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeRequest {
    /// The prompt (for older Claude models).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// Maximum tokens to generate.
    pub max_tokens: i32,
    /// Temperature (0.0 - 1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Top-p sampling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Top-k sampling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i32>,
    /// Stop sequences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    /// Messages (for Claude 3).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub messages: Option<Vec<ClaudeMessage>>,
    /// System prompt (for Claude 3).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
}

/// Anthropic Claude message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeMessage {
    /// The role.
    pub role: ClaudeRole,
    /// The content.
    pub content: Vec<ClaudeContent>,
}

/// Claude role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClaudeRole {
    /// User message.
    User,
    /// Assistant message.
    Assistant,
}

/// Claude content block.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClaudeContent {
    /// Text content.
    Text {
        /// The text.
        text: String,
    },
    /// Image content.
    Image {
        /// The image source.
        source: ClaudeImageSource,
    },
}

/// Claude image source.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeImageSource {
    /// The type (e.g., "base64").
    #[serde(rename = "type")]
    pub source_type: String,
    /// The media type.
    pub media_type: String,
    /// The base64 data.
    pub data: String,
}

/// Anthropic Claude response body.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeResponse {
    /// Completion text (for older models).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion: Option<String>,
    /// Response ID.
    pub id: String,
    /// Model used.
    pub model: String,
    /// Content (for Claude 3).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Vec<ClaudeContent>>,
    /// Stop reason.
    pub stop_reason: Option<String>,
    /// Stop sequence.
    pub stop_sequence: Option<String>,
    /// Usage statistics.
    pub usage: ClaudeUsage,
}

/// Claude usage statistics.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeUsage {
    /// Input tokens.
    pub input_tokens: i32,
    /// Output tokens.
    pub output_tokens: i32,
}

impl ClaudeRequest {
    /// Create a new Claude 3 request with messages.
    pub fn new_claude3(max_tokens: i32, messages: Vec<ClaudeMessage>) -> Self {
        Self {
            prompt: None,
            max_tokens,
            temperature: None,
            top_p: None,
            top_k: None,
            stop_sequences: None,
            messages: Some(messages),
            system: None,
        }
    }

    /// Create a text completion request (older models).
    pub fn new_completion(prompt: impl Into<String>, max_tokens: i32) -> Self {
        Self {
            prompt: Some(prompt.into()),
            max_tokens,
            temperature: None,
            top_p: None,
            top_k: None,
            stop_sequences: None,
            messages: None,
            system: None,
        }
    }

    /// Set the system prompt.
    pub fn system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(system.into());
        self
    }

    /// Set temperature.
    pub fn temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature.clamp(0.0, 1.0));
        self
    }

    /// Set top-p.
    pub fn top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p.clamp(0.0, 1.0));
        self
    }
}

impl ClaudeResponse {
    /// Get the text content from the response.
    pub fn text(&self) -> String {
        if let Some(completion) = &self.completion {
            return completion.clone();
        }

        if let Some(content) = &self.content {
            return content
                .iter()
                .filter_map(|c| match c {
                    ClaudeContent::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("");
        }

        String::new()
    }
}
