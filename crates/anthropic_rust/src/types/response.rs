//! Response types for the Anthropic Messages API.

use super::ContentBlock;
use serde::{Deserialize, Serialize};

/// A response from the Anthropic Messages API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageResponse {
    /// Unique identifier for the message.
    pub id: String,

    /// The object type (always "message").
    #[serde(rename = "type")]
    pub message_type: MessageType,

    /// The role of the author (always "assistant").
    pub role: super::MessageRole,

    /// The content of the message.
    pub content: Vec<ContentBlock>,

    /// The model that generated the response.
    pub model: String,

    /// The reason the response stopped.
    #[serde(rename = "stop_reason")]
    pub stop_reason: Option<StopReason>,

    /// The sequence that caused the response to stop, if any.
    #[serde(rename = "stop_sequence")]
    pub stop_sequence: Option<String>,

    /// Usage statistics for the request.
    pub usage: Usage,
}

impl MessageResponse {
    /// Get the text content of this response.
    pub fn text(&self) -> String {
        self.content
            .iter()
            .filter_map(|block| match block {
                ContentBlock::Text(t) => Some(t.text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }

    /// Check if this response contains any tool use blocks.
    pub fn has_tool_use(&self) -> bool {
        self.content.iter().any(|block| matches!(block, ContentBlock::ToolUse(_)))
    }

    /// Get all tool use blocks from this response.
    pub fn tool_uses(&self) -> Vec<&super::ToolUse> {
        self.content
            .iter()
            .filter_map(|block| match block {
                ContentBlock::ToolUse(tu) => Some(tu),
                _ => None,
            })
            .collect()
    }

    /// Check if the response stopped due to tool use.
    pub fn stopped_for_tool(&self) -> bool {
        matches!(self.stop_reason, Some(StopReason::ToolUse))
    }

    /// Check if the response stopped due to max tokens.
    pub fn stopped_for_max_tokens(&self) -> bool {
        matches!(self.stop_reason, Some(StopReason::MaxTokens))
    }

    /// Check if the response completed naturally.
    pub fn stopped_naturally(&self) -> bool {
        matches!(self.stop_reason, Some(StopReason::EndTurn) | None)
    }
}

/// The type of message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    /// A message from the assistant.
    Message,
}

/// The reason the model stopped generating.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    /// The model completed its turn naturally.
    EndTurn,
    /// The model called a tool.
    ToolUse,
    /// The maximum number of tokens was reached.
    MaxTokens,
    /// A stop sequence was hit.
    StopSequence,
}

impl std::fmt::Display for StopReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StopReason::EndTurn => write!(f, "end_turn"),
            StopReason::ToolUse => write!(f, "tool_use"),
            StopReason::MaxTokens => write!(f, "max_tokens"),
            StopReason::StopSequence => write!(f, "stop_sequence"),
        }
    }
}

/// Token usage statistics.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Usage {
    /// Tokens in the input prompt.
    #[serde(rename = "input_tokens")]
    pub input_tokens: usize,

    /// Tokens in the generated output.
    #[serde(rename = "output_tokens")]
    pub output_tokens: usize,
}

impl Usage {
    /// Total tokens (input + output).
    pub fn total_tokens(&self) -> usize {
        self.input_tokens + self.output_tokens
    }

    /// Create a new usage with the given token counts.
    pub fn new(input: usize, output: usize) -> Self {
        Self {
            input_tokens: input,
            output_tokens: output,
        }
    }
}

impl std::ops::Add for Usage {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            input_tokens: self.input_tokens + other.input_tokens,
            output_tokens: self.output_tokens + other.output_tokens,
        }
    }
}

impl std::iter::Sum for Usage {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Usage::default(), |a, b| a + b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stop_reason_display() {
        assert_eq!(StopReason::EndTurn.to_string(), "end_turn");
        assert_eq!(StopReason::ToolUse.to_string(), "tool_use");
    }

    #[test]
    fn test_usage_math() {
        let u1 = Usage::new(10, 5);
        let u2 = Usage::new(8, 4);
        let total = u1 + u2;
        assert_eq!(total.input_tokens, 18);
        assert_eq!(total.output_tokens, 9);
        assert_eq!(total.total_tokens(), 27);
    }

    #[test]
    fn test_usage_sum() {
        let usages = vec![Usage::new(10, 5), Usage::new(5, 3), Usage::new(2, 1)];
        let total: Usage = usages.into_iter().sum();
        assert_eq!(total.input_tokens, 17);
        assert_eq!(total.output_tokens, 9);
    }

    #[test]
    fn test_response_helpers() {
        let response = MessageResponse {
            id: "msg_123".to_string(),
            message_type: MessageType::Message,
            role: super::super::MessageRole::Assistant,
            content: vec![
                ContentBlock::text("Hello "),
                ContentBlock::text("world!"),
            ],
            model: "claude-3".to_string(),
            stop_reason: Some(StopReason::EndTurn),
            stop_sequence: None,
            usage: Usage::new(10, 5),
        };

        assert_eq!(response.text(), "Hello world!");
        assert!(!response.has_tool_use());
        assert!(response.stopped_naturally());
    }
}
