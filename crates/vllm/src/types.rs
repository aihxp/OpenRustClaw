//! Type definitions for the vLLM API.

use serde::{Deserialize, Serialize};

/// The role of a message author.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// System message.
    System,
    /// User message.
    User,
    /// Assistant message.
    Assistant,
    /// Tool message.
    Tool,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::System => write!(f, "system"),
            Role::User => write!(f, "user"),
            Role::Assistant => write!(f, "assistant"),
            Role::Tool => write!(f, "tool"),
        }
    }
}

/// A chat message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// The role of the message author.
    pub role: Role,
    /// The content of the message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// The name of the author (for tool messages).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Tool calls (for assistant messages).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    /// Tool call ID (for tool messages).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl ChatMessage {
    /// Create a new message.
    pub fn new(role: Role, content: impl Into<String>) -> Self {
        Self {
            role,
            content: Some(content.into()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
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

    /// Create a tool message.
    pub fn tool(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: Role::Tool,
            content: Some(content.into()),
            name: None,
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
        }
    }

    /// Set the name of the author.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
}

/// A tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// The ID of the tool call.
    pub id: String,
    /// The type of the tool call.
    #[serde(rename = "type")]
    pub call_type: String,
    /// The function to call.
    pub function: FunctionCall,
}

/// A function call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    /// The name of the function.
    pub name: String,
    /// The arguments (JSON string).
    pub arguments: String,
}

/// A tool definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// The type of tool.
    #[serde(rename = "type")]
    pub tool_type: String,
    /// The function definition.
    pub function: Function,
}

impl Tool {
    /// Create a new function tool.
    pub fn function(name: impl Into<String>, description: impl Into<String>, parameters: serde_json::Value) -> Self {
        Self {
            tool_type: "function".to_string(),
            function: Function {
                name: name.into(),
                description: description.into(),
                parameters,
            },
        }
    }
}

/// A function definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    /// The name of the function.
    pub name: String,
    /// A description of the function.
    pub description: String,
    /// The parameters schema.
    pub parameters: serde_json::Value,
}

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

/// A chat completion response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    /// Unique identifier.
    pub id: String,
    /// The object type.
    pub object: String,
    /// Unix timestamp.
    pub created: i64,
    /// The model used.
    pub model: String,
    /// The choices.
    pub choices: Vec<ChatChoice>,
    /// Usage statistics.
    pub usage: TokenUsage,
}

impl ChatResponse {
    /// Get the content of the first choice.
    pub fn content(&self) -> &str {
        self.choices
            .first()
            .and_then(|c| c.message.content.as_ref())
            .map(|s| s.as_str())
            .unwrap_or("")
    }

    /// Check if the response has tool calls.
    pub fn has_tool_calls(&self) -> bool {
        self.choices
            .first()
            .and_then(|c| c.message.tool_calls.as_ref())
            .map(|t| !t.is_empty())
            .unwrap_or(false)
    }

    /// Get tool calls from the first choice.
    pub fn tool_calls(&self) -> Option<&Vec<ToolCall>> {
        self.choices
            .first()
            .and_then(|c| c.message.tool_calls.as_ref())
    }

    /// Get the finish reason.
    pub fn finish_reason(&self) -> Option<&str> {
        self.choices
            .first()
            .and_then(|c| c.finish_reason.as_deref())
    }
}

/// A chat choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChoice {
    /// The index.
    pub index: usize,
    /// The message.
    pub message: ChatMessage,
    /// Finish reason.
    pub finish_reason: Option<String>,
}

/// A completion response (legacy).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    /// Unique identifier.
    pub id: String,
    /// The object type.
    pub object: String,
    /// Unix timestamp.
    pub created: i64,
    /// The model used.
    pub model: String,
    /// The choices.
    pub choices: Vec<CompletionChoice>,
    /// Usage statistics.
    pub usage: TokenUsage,
}

impl CompletionResponse {
    /// Get the text of the first choice.
    pub fn text(&self) -> &str {
        self.choices
            .first()
            .map(|c| c.text.as_str())
            .unwrap_or("")
    }

    /// Get the finish reason.
    pub fn finish_reason(&self) -> Option<&str> {
        self.choices
            .first()
            .and_then(|c| c.finish_reason.as_deref())
    }
}

/// A completion choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionChoice {
    /// The index.
    pub index: usize,
    /// The generated text.
    pub text: String,
    /// Logprobs (if requested).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<serde_json::Value>,
    /// Finish reason.
    pub finish_reason: Option<String>,
}

/// An embedding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding {
    /// The index of the embedding.
    pub index: usize,
    /// The embedding vector.
    pub embedding: Vec<f32>,
    /// The object type.
    pub object: String,
}

/// Usage for embeddings.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct EmbeddingUsage {
    /// Tokens used for the prompt.
    #[serde(rename = "prompt_tokens")]
    pub prompt_tokens: usize,
    /// Total tokens.
    #[serde(rename = "total_tokens")]
    pub total_tokens: usize,
}

/// Response format options.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseFormat {
    /// Text response format.
    Text,
    /// JSON object response format.
    JsonObject,
    /// JSON schema response format.
    JsonSchema {
        /// The JSON schema.
        schema: serde_json::Value,
    },
}

/// Logit bias for controlling token generation.
pub type LogitBias = std::collections::HashMap<String, f32>;

/// Finish reason for generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    /// Natural stop at end of text.
    Stop,
    /// Reached maximum token limit.
    Length,
    /// Stopped due to content filter.
    ContentFilter,
    /// Stopped because tool was called.
    ToolCalls,
}

/// Sampling parameters for generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplingParams {
    /// Temperature for sampling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Top-p (nucleus) sampling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Top-k sampling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<usize>,
    /// Presence penalty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    /// Frequency penalty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    /// Repetition penalty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repetition_penalty: Option<f32>,
}

impl Default for SamplingParams {
    fn default() -> Self {
        Self {
            temperature: Some(1.0),
            top_p: Some(1.0),
            top_k: None,
            presence_penalty: None,
            frequency_penalty: None,
            repetition_penalty: None,
        }
    }
}

/// Tool choice options.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolChoice {
    /// No tools should be called.
    None,
    /// The model can choose to call tools.
    Auto,
    /// The model must call a tool.
    Required,
    /// Force a specific tool to be called.
    Function {
        /// The function to call.
        function: FunctionToolChoice,
    },
}

/// Function tool choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionToolChoice {
    /// The name of the function to call.
    pub name: String,
}

impl ToolChoice {
    /// Create an auto tool choice.
    pub fn auto() -> Self {
        Self::Auto
    }

    /// Create a none tool choice.
    pub fn none() -> Self {
        Self::None
    }

    /// Create a required tool choice.
    pub fn required() -> Self {
        Self::Required
    }

    /// Create a specific function tool choice.
    pub fn function(name: impl Into<String>) -> Self {
        Self::Function {
            function: FunctionToolChoice { name: name.into() },
        }
    }
}
