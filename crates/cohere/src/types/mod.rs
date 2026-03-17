//! Common type definitions for the Cohere API.

use serde::{Deserialize, Serialize};

/// A message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// The role of the message author.
    pub role: MessageRole,
    /// The content of the message.
    pub content: String,
}

impl Message {
    /// Create a new message.
    pub fn new(role: MessageRole, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
        }
    }

    /// Create a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self::new(MessageRole::User, content)
    }

    /// Create an assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(MessageRole::Assistant, content)
    }

    /// Create a system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self::new(MessageRole::System, content)
    }

    /// Create a tool message.
    pub fn tool(content: impl Into<String>) -> Self {
        Self::new(MessageRole::Tool, content)
    }

    /// Create a chatbot message (alias for assistant).
    pub fn chatbot(content: impl Into<String>) -> Self {
        Self::new(MessageRole::Chatbot, content)
    }
}

/// The role of a message author.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    /// A message from the user.
    User,
    /// A message from the assistant (Command R/R+).
    Assistant,
    /// A message from the chatbot (legacy Command models).
    Chatbot,
    /// A system message.
    System,
    /// A tool message.
    Tool,
}

/// A document for RAG (Retrieval-Augmented Generation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// Unique identifier for the document.
    pub id: String,
    /// The text content of the document.
    pub text: String,
    /// Optional title of the document.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Additional metadata for the document.
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub extra: Option<serde_json::Map<String, serde_json::Value>>,
}

impl Document {
    /// Create a new document.
    pub fn new(id: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            title: None,
            extra: None,
        }
    }

    /// Set the title of the document.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Add additional metadata.
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.extra
            .get_or_insert_with(serde_json::Map::new)
            .insert(key.into(), value);
        self
    }
}

/// API metadata including billing information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiMeta {
    /// API version used.
    #[serde(rename = "api_version", skip_serializing_if = "Option::is_none")]
    pub api_version: Option<ApiVersion>,
    /// Billed units information.
    #[serde(rename = "billed_units", skip_serializing_if = "Option::is_none")]
    pub billed_units: Option<BilledUnits>,
    /// Token count information.
    #[serde(rename = "tokens", skip_serializing_if = "Option::is_none")]
    pub tokens: Option<TokenInfo>,
}

/// API version information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiVersion {
    /// The API version string.
    pub version: String,
}

/// Billed units information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BilledUnits {
    /// Number of input tokens billed.
    #[serde(rename = "input_tokens", skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<f64>,
    /// Number of output tokens billed.
    #[serde(rename = "output_tokens", skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<f64>,
    /// Total number of tokens billed.
    #[serde(rename = "total_tokens", skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<f64>,
    /// Number of search units billed.
    #[serde(rename = "search_units", skip_serializing_if = "Option::is_none")]
    pub search_units: Option<f64>,
    /// Number of classifications billed.
    #[serde(rename = "classifications", skip_serializing_if = "Option::is_none")]
    pub classifications: Option<f64>,
}

/// Token count information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    /// Number of input tokens.
    #[serde(rename = "input_tokens", skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<usize>,
    /// Number of output tokens.
    #[serde(rename = "output_tokens", skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<usize>,
}

/// Finish reason for generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FinishReason {
    /// The model completed the generation.
    Complete,
    /// The maximum number of tokens was reached.
    MaxTokens,
    /// The stop sequence was encountered.
    StopSequence,
}

/// Connector for RAG (retrieval-augmented generation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connector {
    /// Connector ID (e.g., "web-search", "slack", etc.)
    pub id: String,
    /// Optional user access token for the connector.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_access_token: Option<String>,
    /// Whether to continue generating if the connector fails.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continue_on_failure: Option<bool>,
    /// Additional options for the connector.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<serde_json::Value>,
}

impl Connector {
    /// Create a new connector with the given ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            user_access_token: None,
            continue_on_failure: None,
            options: None,
        }
    }

    /// Set the user access token.
    pub fn with_access_token(mut self, token: impl Into<String>) -> Self {
        self.user_access_token = Some(token.into());
        self
    }

    /// Set whether to continue on failure.
    pub fn continue_on_failure(mut self, value: bool) -> Self {
        self.continue_on_failure = Some(value);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = Message::user("Hello");
        assert_eq!(msg.role, MessageRole::User);
        assert_eq!(msg.content, "Hello");

        let msg = Message::assistant("Hi there!");
        assert_eq!(msg.role, MessageRole::Assistant);
    }

    #[test]
    fn test_document_creation() {
        let doc = Document::new("doc1", "Document content")
            .with_title("My Doc")
            .with_metadata("author", serde_json::json!("John"));

        assert_eq!(doc.id, "doc1");
        assert_eq!(doc.title, Some("My Doc".to_string()));
    }

    #[test]
    fn test_connector() {
        let connector = Connector::new("web-search")
            .with_access_token("token123")
            .continue_on_failure(true);

        assert_eq!(connector.id, "web-search");
        assert_eq!(connector.user_access_token, Some("token123".to_string()));
        assert_eq!(connector.continue_on_failure, Some(true));
    }
}
