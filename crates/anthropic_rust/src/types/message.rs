//! Message types for the Anthropic API.

use serde::{Deserialize, Serialize};

/// The role of a message author.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    /// The user (human).
    User,
    /// The assistant (AI).
    Assistant,
}

impl std::fmt::Display for MessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageRole::User => write!(f, "user"),
            MessageRole::Assistant => write!(f, "assistant"),
        }
    }
}

/// A message in the conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// The role of the message author.
    pub role: MessageRole,
    /// The content of the message.
    #[serde(with = "serde_content")]
    pub content: Vec<ContentBlock>,
}

impl Message {
    /// Create a new message with the given role and text content.
    pub fn new(role: MessageRole, text: impl Into<String>) -> Self {
        Self {
            role,
            content: vec![ContentBlock::text(text)],
        }
    }

    /// Create a new user message.
    pub fn user(text: impl Into<String>) -> Self {
        Self::new(MessageRole::User, text)
    }

    /// Create a new assistant message.
    pub fn assistant(text: impl Into<String>) -> Self {
        Self::new(MessageRole::Assistant, text)
    }

    /// Create a message with multiple content blocks.
    pub fn with_content(role: MessageRole, content: Vec<ContentBlock>) -> Self {
        Self { role, content }
    }

    /// Add a content block to this message.
    pub fn add_block(&mut self, block: ContentBlock) {
        self.content.push(block);
    }

    /// Get the text content of this message (concatenates all text blocks).
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

    /// Check if this message contains any tool use blocks.
    pub fn has_tool_use(&self) -> bool {
        self.content
            .iter()
            .any(|block| matches!(block, ContentBlock::ToolUse(_)))
    }

    /// Get all tool use blocks from this message.
    pub fn tool_uses(&self) -> Vec<&super::ToolUse> {
        self.content
            .iter()
            .filter_map(|block| match block {
                ContentBlock::ToolUse(tu) => Some(tu),
                _ => None,
            })
            .collect()
    }
}

/// A content block in a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Text content.
    Text(TextBlock),
    /// Image content.
    Image(ImageContent),
    /// Tool use request.
    ToolUse(super::ToolUse),
    /// Tool result.
    ToolResult(super::ToolResult),
}

impl ContentBlock {
    /// Create a text content block.
    pub fn text(content: impl Into<String>) -> Self {
        ContentBlock::Text(TextBlock {
            text: content.into(),
        })
    }

    /// Create an image content block from bytes.
    pub fn image(media_type: impl Into<String>, data: impl Into<String>) -> Self {
        ContentBlock::Image(ImageContent {
            source: ImageSource {
                source_type: "base64".to_string(),
                media_type: media_type.into(),
                data: data.into(),
            },
        })
    }

    /// Create a tool use block.
    pub fn tool_use(
        id: impl Into<String>,
        name: impl Into<String>,
        input: serde_json::Value,
    ) -> Self {
        ContentBlock::ToolUse(super::ToolUse {
            id: id.into(),
            name: name.into(),
            input,
        })
    }

    /// Create a tool result block.
    pub fn tool_result(
        tool_use_id: impl Into<String>,
        content: impl Into<String>,
        is_error: bool,
    ) -> Self {
        ContentBlock::ToolResult(super::ToolResult {
            tool_use_id: tool_use_id.into(),
            content: content.into(),
            is_error: Some(is_error),
        })
    }
}

/// A text content block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextBlock {
    /// The text content.
    pub text: String,
}

/// An image content block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageContent {
    /// The image source.
    pub source: ImageSource,
}

/// The source of an image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSource {
    /// The type of source (currently only "base64").
    #[serde(rename = "type")]
    pub source_type: String,
    /// The media type (e.g., "image/jpeg", "image/png", "image/gif", "image/webp").
    pub media_type: String,
    /// The base64-encoded image data.
    pub data: String,
}

/// A delta in a streaming content block.
#[cfg(feature = "streaming")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlockDelta {
    /// Text delta.
    TextDelta(TextDelta),
    /// JSON delta for tool use.
    PartialJson {
        /// The partial JSON string.
        partial_json: String,
    },
}

#[cfg(feature = "streaming")]
impl ContentBlockDelta {
    /// Get the text content if this is a text delta.
    pub fn text(&self) -> &str {
        match self {
            ContentBlockDelta::TextDelta(t) => &t.text,
            ContentBlockDelta::PartialJson { .. } => "",
        }
    }
}

/// A text delta in a stream.
#[cfg(feature = "streaming")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextDelta {
    /// The text delta.
    pub text: String,
}

/// Custom serialization for content (handles both string and array formats).
mod serde_content {
    use super::ContentBlock;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(content: &[ContentBlock], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Always serialize as an array of blocks
        content.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<ContentBlock>, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Handle both single string and array formats
        struct ContentVisitor;

        impl<'de> serde::de::Visitor<'de> for ContentVisitor {
            type Value = Vec<ContentBlock>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("string or array of content blocks")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(vec![ContentBlock::text(value)])
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(vec![ContentBlock::text(value)])
            }

            fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                Vec::<ContentBlock>::deserialize(serde::de::value::SeqAccessDeserializer::new(seq))
            }
        }

        deserializer.deserialize_any(ContentVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = Message::user("Hello");
        assert_eq!(msg.role, MessageRole::User);
        assert_eq!(msg.text(), "Hello");
    }

    #[test]
    fn test_content_block_text() {
        let block = ContentBlock::text("Hello");
        match block {
            ContentBlock::Text(t) => assert_eq!(t.text, "Hello"),
            _ => panic!("Expected text block"),
        }
    }

    #[test]
    fn test_serialize_message() {
        let msg = Message::user("Hello");
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("user"));
        assert!(json.contains("Hello"));
    }

    #[test]
    fn test_deserialize_message_with_string_content() {
        let json = r#"{"role": "user", "content": "Hello"}"#;
        let msg: Message = serde_json::from_str(json).unwrap();
        assert_eq!(msg.role, MessageRole::User);
        assert_eq!(msg.text(), "Hello");
    }

    #[test]
    fn test_deserialize_message_with_array_content() {
        let json = r#"{"role": "assistant", "content": [{"type": "text", "text": "Hi"}]}"#;
        let msg: Message = serde_json::from_str(json).unwrap();
        assert_eq!(msg.role, MessageRole::Assistant);
        assert_eq!(msg.text(), "Hi");
    }
}
