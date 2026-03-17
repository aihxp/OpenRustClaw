//! Conversation and message types for Bedrock.

use serde::{Deserialize, Serialize};

use super::ContentBlock;

/// The role of a message author in a conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConversationRole {
    /// The user (human).
    User,
    /// The assistant (AI).
    Assistant,
}

impl ConversationRole {
    /// Get the role as a string.
    pub fn as_str(&self) -> &'static str {
        match self {
            ConversationRole::User => "user",
            ConversationRole::Assistant => "assistant",
        }
    }
}

impl std::fmt::Display for ConversationRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    /// The role of the message author.
    pub role: ConversationRole,
    /// The content of the message.
    pub content: Vec<ContentBlock>,
}

impl Message {
    /// Create a new user message with text content.
    pub fn user_text(text: impl Into<String>) -> Self {
        Self {
            role: ConversationRole::User,
            content: vec![ContentBlock::text(text)],
        }
    }

    /// Create a new assistant message with text content.
    pub fn assistant_text(text: impl Into<String>) -> Self {
        Self {
            role: ConversationRole::Assistant,
            content: vec![ContentBlock::text(text)],
        }
    }

    /// Create a new message with the given role and content.
    pub fn new(role: ConversationRole, content: Vec<ContentBlock>) -> Self {
        Self { role, content }
    }

    /// Create a new message with a single text content block.
    pub fn text(role: ConversationRole, text: impl Into<String>) -> Self {
        Self {
            role,
            content: vec![ContentBlock::text(text)],
        }
    }

    /// Add a content block to this message.
    pub fn add_content(mut self, block: ContentBlock) -> Self {
        self.content.push(block);
        self
    }

    /// Get the text content of this message (concatenates all text blocks).
    pub fn text_content(&self) -> String {
        self.content
            .iter()
            .filter_map(|block| block.as_text())
            .collect::<Vec<_>>()
            .join("")
    }

    /// Check if this message contains any tool use blocks.
    pub fn has_tool_use(&self) -> bool {
        self.content.iter().any(|block| block.is_tool_use())
    }

    /// Check if this message contains any tool result blocks.
    pub fn has_tool_result(&self) -> bool {
        self.content.iter().any(|block| block.is_tool_result())
    }

    /// Get the role of this message.
    pub fn role(&self) -> ConversationRole {
        self.role
    }
}

/// Stop reason for model generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    /// The model reached a natural stopping point.
    EndTurn,
    /// The model invoked a tool.
    ToolUse,
    /// The maximum number of tokens was reached.
    MaxTokens,
    /// The model was stopped.
    StopSequence,
    /// Guardrail intervention occurred.
    GuardrailIntervened,
    /// Content was filtered.
    ContentFiltered,
}

/// Token usage statistics.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsage {
    /// The number of tokens in the prompt.
    pub input_tokens: i32,
    /// The number of tokens in the completion.
    pub output_tokens: i32,
    /// The total number of tokens used.
    pub total_tokens: i32,
}

impl TokenUsage {
    /// Create new token usage.
    pub fn new(input_tokens: i32, output_tokens: i32) -> Self {
        Self {
            input_tokens,
            output_tokens,
            total_tokens: input_tokens + output_tokens,
        }
    }

    /// Add another token usage to this one.
    pub fn add(&mut self, other: &TokenUsage) {
        self.input_tokens += other.input_tokens;
        self.output_tokens += other.output_tokens;
        self.total_tokens += other.total_tokens;
    }
}

/// A conversation that can be used for the Converse API.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InlineConversation {
    /// The messages in the conversation.
    pub messages: Vec<Message>,
}

impl InlineConversation {
    /// Create a new empty conversation.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a conversation with initial messages.
    pub fn with_messages(messages: Vec<Message>) -> Self {
        Self { messages }
    }

    /// Add a message to the conversation.
    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    /// Add a user text message.
    pub fn add_user_message(&mut self, text: impl Into<String>) {
        self.messages.push(Message::user_text(text));
    }

    /// Add an assistant text message.
    pub fn add_assistant_message(&mut self, text: impl Into<String>) {
        self.messages.push(Message::assistant_text(text));
    }

    /// Get the number of messages.
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Check if the conversation is empty.
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Get the last message if any.
    pub fn last_message(&self) -> Option<&Message> {
        self.messages.last()
    }

    /// Get a mutable reference to the last message.
    pub fn last_message_mut(&mut self) -> Option<&mut Message> {
        self.messages.last_mut()
    }

    /// Clear all messages.
    pub fn clear(&mut self) {
        self.messages.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ContentBlock;

    #[test]
    fn test_conversation_role() {
        assert_eq!(ConversationRole::User.as_str(), "user");
        assert_eq!(ConversationRole::Assistant.as_str(), "assistant");
    }

    #[test]
    fn test_message_creation() {
        let msg = Message::user_text("Hello");
        assert_eq!(msg.role, ConversationRole::User);
        assert_eq!(msg.text_content(), "Hello");

        let msg = Message::assistant_text("Hi there!");
        assert_eq!(msg.role, ConversationRole::Assistant);
        assert_eq!(msg.text_content(), "Hi there!");
    }

    #[test]
    fn test_message_with_content() {
        let msg = Message::new(
            ConversationRole::User,
            vec![
                ContentBlock::text("What's in this image?"),
                ContentBlock::image_from_s3("mybucket", "myimage.jpg"),
            ],
        );
        assert_eq!(msg.text_content(), "What's in this image?");
        assert!(!msg.has_tool_use());
    }

    #[test]
    fn test_message_tool_use() {
        let msg = Message::new(
            ConversationRole::Assistant,
            vec![ContentBlock::tool_use(
                "tool_1",
                "search",
                serde_json::json!({"query": "test"}),
            )],
        );
        assert!(msg.has_tool_use());
    }

    #[test]
    fn test_token_usage() {
        let usage = TokenUsage::new(100, 50);
        assert_eq!(usage.input_tokens, 100);
        assert_eq!(usage.output_tokens, 50);
        assert_eq!(usage.total_tokens, 150);

        let mut usage2 = TokenUsage::new(50, 25);
        usage2.add(&usage);
        assert_eq!(usage2.input_tokens, 150);
        assert_eq!(usage2.output_tokens, 75);
        assert_eq!(usage2.total_tokens, 225);
    }

    #[test]
    fn test_conversation() {
        let mut conv = InlineConversation::new();
        assert!(conv.is_empty());

        conv.add_user_message("Hello");
        conv.add_assistant_message("Hi!");

        assert_eq!(conv.len(), 2);
        assert!(!conv.is_empty());

        let last = conv.last_message().unwrap();
        assert_eq!(last.role, ConversationRole::Assistant);

        conv.clear();
        assert!(conv.is_empty());
    }
}
