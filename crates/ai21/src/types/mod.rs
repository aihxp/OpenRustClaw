//! Common type definitions for the AI21 API.

use serde::{Deserialize, Serialize};

/// A message in a conversation (Jamba models).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// The role of the message author.
    pub role: MessageRole,
    /// The content of the message.
    pub content: String,
    /// Optional name for the message author.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl Message {
    /// Create a new message.
    pub fn new(role: MessageRole, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
            name: None,
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

    /// Set the name for the message author.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
}

/// The role of a message author.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    /// A system message.
    System,
    /// A message from the user.
    User,
    /// A message from the assistant.
    Assistant,
}

/// A document for RAG (Retrieval-Augmented Generation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// Unique identifier for the document.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// The text content of the document.
    pub text: String,
    /// Optional metadata for the document.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl Document {
    /// Create a new document.
    pub fn new(id: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            id: Some(id.into()),
            text: text.into(),
            metadata: None,
        }
    }

    /// Create a document without an ID.
    pub fn from_text(text: impl Into<String>) -> Self {
        Self {
            id: None,
            text: text.into(),
            metadata: None,
        }
    }

    /// Set the document metadata.
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Usage information for API requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    /// Number of tokens in the prompt.
    #[serde(rename = "prompt_tokens")]
    pub prompt_tokens: usize,
    /// Number of tokens in the completion.
    #[serde(rename = "completion_tokens")]
    pub completion_tokens: usize,
    /// Total number of tokens.
    #[serde(rename = "total_tokens")]
    pub total_tokens: usize,
}

/// Finish reason for generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    /// The model completed the generation naturally.
    Stop,
    /// The maximum number of tokens was reached.
    Length,
    /// Content was filtered.
    ContentFilter,
}

/// Penalty configuration for text generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Penalty {
    /// Scale of the penalty (0.0 to 5.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<f32>,
    /// Whether to apply penalty to numbers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apply_to_numbers: Option<bool>,
    /// Whether to apply penalty to punctuation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apply_to_punctuation: Option<bool>,
    /// Whether to apply penalty to stop words.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apply_to_stop_words: Option<bool>,
    /// Whether to apply penalty to whitespace.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apply_to_whitespace: Option<bool>,
    /// Whether to apply penalty to emojis.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apply_to_emojis: Option<bool>,
}

impl Penalty {
    /// Create a new penalty with the given scale.
    pub fn new(scale: f32) -> Self {
        Self {
            scale: Some(scale.clamp(0.0, 5.0)),
            apply_to_numbers: None,
            apply_to_punctuation: None,
            apply_to_stop_words: None,
            apply_to_whitespace: None,
            apply_to_emojis: None,
        }
    }

    /// Apply penalty to numbers.
    pub fn with_numbers(mut self, apply: bool) -> Self {
        self.apply_to_numbers = Some(apply);
        self
    }

    /// Apply penalty to punctuation.
    pub fn with_punctuation(mut self, apply: bool) -> Self {
        self.apply_to_punctuation = Some(apply);
        self
    }

    /// Apply penalty to stop words.
    pub fn with_stop_words(mut self, apply: bool) -> Self {
        self.apply_to_stop_words = Some(apply);
        self
    }

    /// Apply penalty to whitespace.
    pub fn with_whitespace(mut self, apply: bool) -> Self {
        self.apply_to_whitespace = Some(apply);
        self
    }

    /// Apply penalty to emojis.
    pub fn with_emojis(mut self, apply: bool) -> Self {
        self.apply_to_emojis = Some(apply);
        self
    }
}

/// Response format configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseFormat {
    /// The type of response format.
    #[serde(rename = "type")]
    pub format_type: String,
}

impl ResponseFormat {
    /// Create a JSON object response format.
    pub fn json_object() -> Self {
        Self {
            format_type: "json_object".to_string(),
        }
    }

    /// Create a text response format (default).
    pub fn text() -> Self {
        Self {
            format_type: "text".to_string(),
        }
    }
}

/// Tool definition for function calling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// The type of tool (always "function").
    #[serde(rename = "type")]
    pub tool_type: String,
    /// The function definition.
    pub function: ToolFunction,
}

impl Tool {
    /// Create a new tool.
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            tool_type: "function".to_string(),
            function: ToolFunction {
                name: name.into(),
                description: description.into(),
                parameters: None,
            },
        }
    }

    /// Set the parameters schema for the tool.
    pub fn with_parameters(mut self, parameters: serde_json::Value) -> Self {
        self.function.parameters = Some(parameters);
        self
    }
}

/// Function definition for a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFunction {
    /// The name of the function.
    pub name: String,
    /// A description of what the function does.
    pub description: String,
    /// The JSON schema for the function parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
}

/// A tool call in a response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// The ID of the tool call.
    pub id: String,
    /// The type of tool call (always "function").
    #[serde(rename = "type")]
    pub call_type: String,
    /// The function call details.
    pub function: ToolCallFunction,
}

/// Function call details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallFunction {
    /// The name of the function to call.
    pub name: String,
    /// The arguments to pass to the function (JSON string).
    pub arguments: String,
}

impl ToolCallFunction {
    /// Parse the arguments as JSON.
    pub fn parse_arguments(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::from_str(&self.arguments)
    }
}

/// Tool call result for sending back to the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// The role of the message (always "tool").
    pub role: String,
    /// The content of the tool result.
    pub content: String,
    /// The ID of the tool call this is responding to.
    pub tool_call_id: String,
}

impl ToolResult {
    /// Create a new tool result.
    pub fn new(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: "tool".to_string(),
            content: content.into(),
            tool_call_id: tool_call_id.into(),
        }
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
        let doc = Document::new("doc1", "Document content");
        assert_eq!(doc.id, Some("doc1".to_string()));
        assert_eq!(doc.text, "Document content");

        let doc = Document::from_text("Simple text");
        assert_eq!(doc.id, None);
    }

    #[test]
    fn test_penalty_builder() {
        let penalty = Penalty::new(2.0)
            .with_numbers(true)
            .with_punctuation(false);
        
        assert_eq!(penalty.scale, Some(2.0));
        assert_eq!(penalty.apply_to_numbers, Some(true));
        assert_eq!(penalty.apply_to_punctuation, Some(false));
    }

    #[test]
    fn test_tool_creation() {
        let tool = Tool::new("get_weather", "Get weather information");
        assert_eq!(tool.tool_type, "function");
        assert_eq!(tool.function.name, "get_weather");
        assert_eq!(tool.function.description, "Get weather information");
    }

    #[test]
    fn test_tool_result() {
        let result = ToolResult::new("call_123", "The weather is sunny");
        assert_eq!(result.role, "tool");
        assert_eq!(result.tool_call_id, "call_123");
        assert_eq!(result.content, "The weather is sunny");
    }
}
