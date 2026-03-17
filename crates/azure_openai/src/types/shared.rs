//! Shared types for the Azure OpenAI API.

use serde::{Deserialize, Serialize};

/// Token usage statistics.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Tokens in the prompt.
    #[serde(rename = "prompt_tokens")]
    pub prompt_tokens: usize,

    /// Tokens in the completion.
    #[serde(rename = "completion_tokens")]
    pub completion_tokens: usize,

    /// Total tokens.
    #[serde(rename = "total_tokens")]
    pub total_tokens: usize,
}

impl TokenUsage {
    /// Create new usage statistics.
    pub fn new(prompt: usize, completion: usize) -> Self {
        Self {
            prompt_tokens: prompt,
            completion_tokens: completion,
            total_tokens: prompt + completion,
        }
    }

    /// Get the total tokens.
    pub fn total(&self) -> usize {
        self.total_tokens
    }
}

impl std::ops::Add for TokenUsage {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            prompt_tokens: self.prompt_tokens + other.prompt_tokens,
            completion_tokens: self.completion_tokens + other.completion_tokens,
            total_tokens: self.total_tokens + other.total_tokens,
        }
    }
}

/// The reason the model stopped generating.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    /// The model hit a natural stop point or a provided stop sequence.
    Stop,
    /// The maximum number of tokens specified in the request was reached.
    Length,
    /// The content was flagged by content filtering.
    ContentFilter,
    /// The model called a tool.
    ToolCalls,
}

impl std::fmt::Display for FinishReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FinishReason::Stop => write!(f, "stop"),
            FinishReason::Length => write!(f, "length"),
            FinishReason::ContentFilter => write!(f, "content_filter"),
            FinishReason::ToolCalls => write!(f, "tool_calls"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usage_math() {
        let u1 = TokenUsage::new(10, 5);
        let u2 = TokenUsage::new(8, 4);
        let total = u1 + u2;
        assert_eq!(total.prompt_tokens, 18);
        assert_eq!(total.completion_tokens, 9);
        assert_eq!(total.total(), 27);
    }

    #[test]
    fn test_finish_reason_display() {
        assert_eq!(FinishReason::Stop.to_string(), "stop");
        assert_eq!(FinishReason::ToolCalls.to_string(), "tool_calls");
    }
}
