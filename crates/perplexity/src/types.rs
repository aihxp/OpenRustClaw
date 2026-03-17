//! Common type definitions for the Perplexity API.

use serde::{Deserialize, Serialize};

/// A message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// The role of the message author.
    pub role: Role,
    /// The content of the message.
    pub content: String,
}

impl Message {
    /// Create a new message.
    pub fn new(role: Role, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
        }
    }

    /// Create a system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self::new(Role::System, content)
    }

    /// Create a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self::new(Role::User, content)
    }

    /// Create an assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(Role::Assistant, content)
    }
}

/// The role of a message author.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// A system message to guide the model.
    System,
    /// A message from the user.
    User,
    /// A message from the assistant.
    Assistant,
}

/// A citation/source in the response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    /// The index of the citation.
    pub index: usize,
    /// The URL of the source.
    pub url: String,
    /// The title of the source (if available).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// The date published (if available).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published_date: Option<String>,
}

/// Token usage information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
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

/// A related question suggested by the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedQuestion {
    /// The suggested question text.
    pub question: String,
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

/// Finish reason for generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    /// The model stopped naturally.
    Stop,
    /// The maximum number of tokens was reached.
    Length,
    /// A stop sequence was encountered.
    #[serde(rename = "stop_sequence")]
    StopSequence,
}

/// Tool definition for function calling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// The type of tool (always "function" for Perplexity).
    #[serde(rename = "type")]
    pub tool_type: String,
    /// The function definition.
    pub function: Function,
}

impl Tool {
    /// Create a new tool definition.
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            tool_type: "function".to_string(),
            function: Function {
                name: name.into(),
                description: description.into(),
                parameters: None,
            },
        }
    }

    /// Set the parameters schema.
    pub fn with_parameters(mut self, parameters: serde_json::Value) -> Self {
        self.function.parameters = Some(parameters);
        self
    }
}

/// Function definition for tool use.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    /// The name of the function.
    pub name: String,
    /// A description of what the function does.
    pub description: String,
    /// JSON Schema for the function parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
}

/// A tool call in the response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// The ID of the tool call.
    pub id: String,
    /// The type of tool (always "function").
    #[serde(rename = "type")]
    pub call_type: String,
    /// The function call details.
    pub function: FunctionCall,
}

/// Function call details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    /// The name of the function to call.
    pub name: String,
    /// The arguments to pass to the function (JSON string).
    pub arguments: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = Message::user("Hello");
        assert_eq!(msg.role, Role::User);
        assert_eq!(msg.content, "Hello");

        let msg = Message::system("You are helpful");
        assert_eq!(msg.role, Role::System);
        assert_eq!(msg.content, "You are helpful");
    }

    #[test]
    fn test_response_format() {
        let json_format = ResponseFormat::json_object();
        assert_eq!(json_format.format_type, "json_object");

        let text_format = ResponseFormat::text();
        assert_eq!(text_format.format_type, "text");
    }

    #[test]
    fn test_tool_creation() {
        let tool = Tool::new("get_weather", "Get weather information");
        assert_eq!(tool.tool_type, "function");
        assert_eq!(tool.function.name, "get_weather");
    }

    #[test]
    fn test_citation() {
        let citation = Citation {
            index: 1,
            url: "https://example.com".to_string(),
            title: Some("Example".to_string()),
            published_date: Some("2024-01-01".to_string()),
        };
        assert_eq!(citation.index, 1);
        assert_eq!(citation.url, "https://example.com");
    }
}
