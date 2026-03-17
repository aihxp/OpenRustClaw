//! Type definitions for the Groq API.

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
    /// Tool calls made by the assistant.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    /// The tool call ID this message is responding to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl ChatMessage {
    /// Create a new message with the given role and content.
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

    /// Create an assistant message with tool calls.
    pub fn assistant_with_tools(content: impl Into<String>, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: Role::Assistant,
            content: Some(content.into()),
            name: None,
            tool_calls: Some(tool_calls),
            tool_call_id: None,
        }
    }
}

/// A tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// The ID of the tool call.
    pub id: String,
    /// The type of the tool call (always "function").
    #[serde(rename = "type")]
    pub call_type: String,
    /// The function to call.
    pub function: FunctionCall,
}

/// A function call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    /// The name of the function to call.
    pub name: String,
    /// The arguments to pass to the function (JSON string).
    pub arguments: String,
}

impl FunctionCall {
    /// Parse the arguments as JSON.
    pub fn parse_arguments(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::from_str(&self.arguments)
    }
}

/// A tool definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// The type of tool (always "function").
    #[serde(rename = "type")]
    pub tool_type: String,
    /// The function definition.
    pub function: Function,
}

impl Tool {
    /// Create a new tool from a function definition.
    pub fn function(function: Function) -> Self {
        Self {
            tool_type: "function".to_string(),
            function,
        }
    }
}

/// A function definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    /// The name of the function.
    pub name: String,
    /// A description of what the function does.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The parameters the function accepts (JSON Schema).
    pub parameters: serde_json::Value,
}

impl Function {
    /// Create a new function definition.
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: Some(description.into()),
            parameters: serde_json::json!({"type": "object"}),
        }
    }

    /// Set the parameters schema.
    pub fn parameters(mut self, params: serde_json::Value) -> Self {
        self.parameters = params;
        self
    }

    /// Builder for function parameters.
    pub fn builder(name: impl Into<String>, description: impl Into<String>) -> FunctionBuilder {
        FunctionBuilder::new(name, description)
    }
}

/// Builder for function definitions.
#[derive(Debug, Clone)]
pub struct FunctionBuilder {
    name: String,
    description: String,
    properties: Vec<(&'static str, serde_json::Value)>,
    required: Vec<&'static str>,
}

impl FunctionBuilder {
    /// Create a new function builder.
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            properties: Vec::new(),
            required: Vec::new(),
        }
    }

    /// Add a string property.
    pub fn string_property(
        mut self,
        name: &'static str,
        description: impl Into<String>,
        required: bool,
    ) -> Self {
        let prop = serde_json::json!({
            "type": "string",
            "description": description.into(),
        });
        self.properties.push((name, prop));
        if required {
            self.required.push(name);
        }
        self
    }

    /// Add an integer property.
    pub fn integer_property(
        mut self,
        name: &'static str,
        description: impl Into<String>,
        required: bool,
    ) -> Self {
        let prop = serde_json::json!({
            "type": "integer",
            "description": description.into(),
        });
        self.properties.push((name, prop));
        if required {
            self.required.push(name);
        }
        self
    }

    /// Add a number property.
    pub fn number_property(
        mut self,
        name: &'static str,
        description: impl Into<String>,
        required: bool,
    ) -> Self {
        let prop = serde_json::json!({
            "type": "number",
            "description": description.into(),
        });
        self.properties.push((name, prop));
        if required {
            self.required.push(name);
        }
        self
    }

    /// Add a boolean property.
    pub fn boolean_property(
        mut self,
        name: &'static str,
        description: impl Into<String>,
        required: bool,
    ) -> Self {
        let prop = serde_json::json!({
            "type": "boolean",
            "description": description.into(),
        });
        self.properties.push((name, prop));
        if required {
            self.required.push(name);
        }
        self
    }

    /// Add an enum property.
    pub fn enum_property(
        mut self,
        name: &'static str,
        description: impl Into<String>,
        variants: Vec<&'static str>,
        required: bool,
    ) -> Self {
        let prop = serde_json::json!({
            "type": "string",
            "description": description.into(),
            "enum": variants,
        });
        self.properties.push((name, prop));
        if required {
            self.required.push(name);
        }
        self
    }

    /// Add an array property.
    pub fn array_property(
        mut self,
        name: &'static str,
        description: impl Into<String>,
        item_type: serde_json::Value,
        required: bool,
    ) -> Self {
        let prop = serde_json::json!({
            "type": "array",
            "description": description.into(),
            "items": item_type,
        });
        self.properties.push((name, prop));
        if required {
            self.required.push(name);
        }
        self
    }

    /// Add an object property.
    pub fn object_property(
        mut self,
        name: &'static str,
        description: impl Into<String>,
        properties: serde_json::Value,
        required: bool,
    ) -> Self {
        let prop = serde_json::json!({
            "type": "object",
            "description": description.into(),
            "properties": properties,
        });
        self.properties.push((name, prop));
        if required {
            self.required.push(name);
        }
        self
    }

    /// Build the function.
    pub fn build(self) -> Function {
        let properties: serde_json::Map<String, serde_json::Value> = self
            .properties
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();

        let mut params = serde_json::json!({
            "type": "object",
            "properties": properties,
        });

        if !self.required.is_empty() {
            params["required"] = serde_json::json!(self.required);
        }

        Function {
            name: self.name,
            description: Some(self.description),
            parameters: params,
        }
    }
}

/// A chat completion response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    /// Unique identifier for the completion.
    pub id: String,
    /// The object type (always "chat.completion").
    pub object: String,
    /// The Unix timestamp when the completion was created.
    pub created: i64,
    /// The model used for completion.
    pub model: String,
    /// The list of completion choices.
    pub choices: Vec<ChatChoice>,
    /// Usage statistics.
    pub usage: TokenUsage,
}

impl ChatResponse {
    /// Get the content of the first choice.
    pub fn content(&self) -> Option<&str> {
        self.choices
            .first()
            .and_then(|c| c.message.content.as_deref())
    }

    /// Get the message of the first choice.
    pub fn message(&self) -> Option<&ChatMessage> {
        self.choices.first().map(|c| &c.message)
    }

    /// Get the finish reason of the first choice.
    pub fn finish_reason(&self) -> Option<FinishReason> {
        self.choices.first().and_then(|c| c.finish_reason)
    }

    /// Check if the response contains tool calls.
    pub fn has_tool_calls(&self) -> bool {
        self.choices
            .first()
            .and_then(|c| c.message.tool_calls.as_ref())
            .map(|t| !t.is_empty())
            .unwrap_or(false)
    }

    /// Get tool calls from the first choice.
    pub fn tool_calls(&self) -> Option<&[ToolCall]> {
        self.choices
            .first()
            .and_then(|c| c.message.tool_calls.as_deref())
    }
}

/// A chat completion choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChoice {
    /// The index of this choice.
    pub index: usize,
    /// The message.
    pub message: ChatMessage,
    /// The reason the completion finished.
    pub finish_reason: Option<FinishReason>,
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

/// Groq model identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GroqModel {
    /// Llama 3 8B (8192 context).
    Llama3_8b,
    /// Llama 3 70B (8192 context).
    Llama3_70b,
    /// Llama 3.1 8B Instant (131072 context).
    Llama3_1_8bInstant,
    /// Llama 3.1 70B Versatile (131072 context).
    Llama3_1_70bVersatile,
    /// Llama 3.1 405B Reasoning (131072 context).
    Llama3_1_405bReasoning,
    /// Mixtral 8x7B (32768 context).
    Mixtral8x7b,
    /// Gemma 7B Instruct (8192 context).
    Gemma7bIt,
    /// Gemma2 9B Instruct (8192 context).
    Gemma2_9bIt,
    /// Whisper Large V3 (for audio transcription).
    WhisperLargeV3,
}

impl GroqModel {
    /// Get the model identifier string.
    pub fn as_str(&self) -> &'static str {
        match self {
            GroqModel::Llama3_8b => "llama3-8b-8192",
            GroqModel::Llama3_70b => "llama3-70b-8192",
            GroqModel::Llama3_1_8bInstant => "llama-3.1-8b-instant",
            GroqModel::Llama3_1_70bVersatile => "llama-3.1-70b-versatile",
            GroqModel::Llama3_1_405bReasoning => "llama-3.1-405b-reasoning",
            GroqModel::Mixtral8x7b => "mixtral-8x7b-32768",
            GroqModel::Gemma7bIt => "gemma-7b-it",
            GroqModel::Gemma2_9bIt => "gemma2-9b-it",
            GroqModel::WhisperLargeV3 => "whisper-large-v3",
        }
    }

    /// Get the maximum context window for this model.
    pub fn max_context_tokens(&self) -> usize {
        match self {
            GroqModel::Llama3_8b => 8_192,
            GroqModel::Llama3_70b => 8_192,
            GroqModel::Llama3_1_8bInstant => 131_072,
            GroqModel::Llama3_1_70bVersatile => 131_072,
            GroqModel::Llama3_1_405bReasoning => 131_072,
            GroqModel::Mixtral8x7b => 32_768,
            GroqModel::Gemma7bIt => 8_192,
            GroqModel::Gemma2_9bIt => 8_192,
            GroqModel::WhisperLargeV3 => 0, // N/A for audio models
        }
    }

    /// Check if this is an audio model.
    pub fn is_audio(&self) -> bool {
        matches!(self, GroqModel::WhisperLargeV3)
    }

    /// Check if this is a reasoning model.
    pub fn is_reasoning(&self) -> bool {
        matches!(self, GroqModel::Llama3_1_405bReasoning)
    }
}

impl std::fmt::Display for GroqModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for GroqModel {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "llama3-8b-8192" => Ok(GroqModel::Llama3_8b),
            "llama3-70b-8192" => Ok(GroqModel::Llama3_70b),
            "llama-3.1-8b-instant" => Ok(GroqModel::Llama3_1_8bInstant),
            "llama-3.1-70b-versatile" => Ok(GroqModel::Llama3_1_70bVersatile),
            "llama-3.1-405b-reasoning" => Ok(GroqModel::Llama3_1_405bReasoning),
            "mixtral-8x7b-32768" => Ok(GroqModel::Mixtral8x7b),
            "gemma-7b-it" => Ok(GroqModel::Gemma7bIt),
            "gemma2-9b-it" => Ok(GroqModel::Gemma2_9bIt),
            "whisper-large-v3" => Ok(GroqModel::WhisperLargeV3),
            _ => Err(format!("Unknown model: {s}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = ChatMessage::user("Hello");
        assert_eq!(msg.role, Role::User);
        assert_eq!(msg.content, Some("Hello".to_string()));
    }

    #[test]
    fn test_function_builder() {
        let func = Function::builder("get_weather", "Get weather")
            .string_property("location", "City name", true)
            .enum_property(
                "unit",
                "Temperature unit",
                vec!["celsius", "fahrenheit"],
                false,
            )
            .build();

        assert_eq!(func.name, "get_weather");
        assert!(func.parameters.get("required").is_some());
    }

    #[test]
    fn test_parse_arguments() {
        let call = FunctionCall {
            name: "test".to_string(),
            arguments: r#"{"foo": "bar", "num": 42}"#.to_string(),
        };

        let args = call.parse_arguments().unwrap();
        assert_eq!(args["foo"], "bar");
        assert_eq!(args["num"], 42);
    }

    #[test]
    fn test_chat_response_helpers() {
        let response = ChatResponse {
            id: "chatcmpl-123".to_string(),
            object: "chat.completion".to_string(),
            created: 1234567890,
            model: "llama3-8b-8192".to_string(),
            choices: vec![ChatChoice {
                index: 0,
                message: ChatMessage::assistant("Hello!"),
                finish_reason: Some(FinishReason::Stop),
            }],
            usage: TokenUsage::new(10, 5),
        };

        assert_eq!(response.content(), Some("Hello!"));
        assert!(!response.has_tool_calls());
    }

    #[test]
    fn test_model_as_str() {
        assert_eq!(GroqModel::Llama3_8b.as_str(), "llama3-8b-8192");
        assert_eq!(
            GroqModel::Llama3_1_70bVersatile.as_str(),
            "llama-3.1-70b-versatile"
        );
        assert_eq!(GroqModel::Mixtral8x7b.as_str(), "mixtral-8x7b-32768");
        assert_eq!(GroqModel::WhisperLargeV3.as_str(), "whisper-large-v3");
    }

    #[test]
    fn test_model_from_str() {
        assert_eq!(
            "llama3-8b-8192".parse::<GroqModel>().unwrap(),
            GroqModel::Llama3_8b
        );
        assert_eq!(
            "mixtral-8x7b-32768".parse::<GroqModel>().unwrap(),
            GroqModel::Mixtral8x7b
        );
    }

    #[test]
    fn test_model_max_context() {
        assert_eq!(GroqModel::Llama3_8b.max_context_tokens(), 8192);
        assert_eq!(GroqModel::Llama3_1_8bInstant.max_context_tokens(), 131072);
        assert_eq!(GroqModel::Mixtral8x7b.max_context_tokens(), 32768);
    }

    #[test]
    fn test_model_is_audio() {
        assert!(GroqModel::WhisperLargeV3.is_audio());
        assert!(!GroqModel::Llama3_8b.is_audio());
    }

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
