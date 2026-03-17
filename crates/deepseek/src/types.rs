//! Type definitions for the DeepSeek API.

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
    /// Reasoning content from the model (DeepSeek-R1).
    /// This contains the chain-of-thought reasoning process.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
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
            reasoning_content: None,
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

    /// Create an assistant message with reasoning content (DeepSeek-R1).
    pub fn assistant_with_reasoning(
        content: impl Into<String>,
        reasoning_content: impl Into<String>,
    ) -> Self {
        Self {
            role: Role::Assistant,
            content: Some(content.into()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: Some(reasoning_content.into()),
        }
    }

    /// Create a tool message.
    pub fn tool(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: Role::Tool,
            content: Some(content.into()),
            name: None,
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
            reasoning_content: None,
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
            reasoning_content: None,
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
        self.choices.first().and_then(|c| c.message.content.as_deref())
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

    /// Get the reasoning content from the first choice (DeepSeek-R1).
    /// This contains the model's chain-of-thought reasoning.
    pub fn reasoning_content(&self) -> Option<&str> {
        self.choices
            .first()
            .and_then(|c| c.message.reasoning_content.as_deref())
    }

    /// Check if this response includes reasoning content.
    pub fn has_reasoning(&self) -> bool {
        self.reasoning_content().is_some()
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

/// DeepSeek model identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeepSeekModel {
    /// DeepSeek-V3 - General purpose chat model (default).
    DeepSeekChat,
    /// DeepSeek-R1 - Reasoning model with chain-of-thought.
    DeepSeekReasoner,
    /// DeepSeek-Coder - Code generation model.
    DeepSeekCoder,
}

impl DeepSeekModel {
    /// Get the model identifier string.
    pub fn as_str(&self) -> &'static str {
        match self {
            DeepSeekModel::DeepSeekChat => "deepseek-chat",
            DeepSeekModel::DeepSeekReasoner => "deepseek-reasoner",
            DeepSeekModel::DeepSeekCoder => "deepseek-coder",
        }
    }

    /// Get the maximum context window for this model.
    pub fn max_context_tokens(&self) -> usize {
        match self {
            DeepSeekModel::DeepSeekChat => 64_000,
            DeepSeekModel::DeepSeekReasoner => 64_000,
            DeepSeekModel::DeepSeekCoder => 64_000,
        }
    }

    /// Get the maximum output tokens for this model.
    pub fn max_output_tokens(&self) -> usize {
        match self {
            DeepSeekModel::DeepSeekChat => 8_192,
            DeepSeekModel::DeepSeekReasoner => 8_192,
            DeepSeekModel::DeepSeekCoder => 8_192,
        }
    }

    /// Check if this is a reasoning model.
    /// Reasoning models (like DeepSeek-R1) provide chain-of-thought
    /// in the `reasoning_content` field.
    pub fn is_reasoning(&self) -> bool {
        matches!(self, DeepSeekModel::DeepSeekReasoner)
    }

    /// Check if this is a code model.
    pub fn is_code(&self) -> bool {
        matches!(self, DeepSeekModel::DeepSeekCoder)
    }
}

impl std::fmt::Display for DeepSeekModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for DeepSeekModel {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "deepseek-chat" => Ok(DeepSeekModel::DeepSeekChat),
            "deepseek-reasoner" => Ok(DeepSeekModel::DeepSeekReasoner),
            "deepseek-coder" => Ok(DeepSeekModel::DeepSeekCoder),
            _ => Err(format!("Unknown model: {s}")),
        }
    }
}

/// A model returned by the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    /// The model identifier.
    pub id: String,
    /// The object type (always "model").
    pub object: String,
    /// The Unix timestamp when the model was created.
    pub created: i64,
    /// The organization that owns the model.
    pub owned_by: String,
}

/// Response from listing models.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListModelsResponse {
    /// The object type (always "list").
    pub object: String,
    /// The list of models.
    pub data: Vec<Model>,
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
    fn test_message_with_reasoning() {
        let msg = ChatMessage::assistant_with_reasoning(
            "The answer is 42.",
            "Let me think step by step...",
        );
        assert_eq!(msg.role, Role::Assistant);
        assert_eq!(msg.content, Some("The answer is 42.".to_string()));
        assert_eq!(msg.reasoning_content, Some("Let me think step by step...".to_string()));
    }

    #[test]
    fn test_function_builder() {
        let func = Function::builder("get_weather", "Get weather")
            .string_property("location", "City name", true)
            .enum_property("unit", "Temperature unit", vec!["celsius", "fahrenheit"], false)
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
            model: "deepseek-chat".to_string(),
            choices: vec![ChatChoice {
                index: 0,
                message: ChatMessage::assistant("Hello!"),
                finish_reason: Some(FinishReason::Stop),
            }],
            usage: TokenUsage::new(10, 5),
        };

        assert_eq!(response.content(), Some("Hello!"));
        assert!(!response.has_tool_calls());
        assert!(!response.has_reasoning());
    }

    #[test]
    fn test_chat_response_with_reasoning() {
        let response = ChatResponse {
            id: "chatcmpl-123".to_string(),
            object: "chat.completion".to_string(),
            created: 1234567890,
            model: "deepseek-reasoner".to_string(),
            choices: vec![ChatChoice {
                index: 0,
                message: ChatMessage::assistant_with_reasoning(
                    "The answer is 42.",
                    "Let me think...",
                ),
                finish_reason: Some(FinishReason::Stop),
            }],
            usage: TokenUsage::new(10, 5),
        };

        assert_eq!(response.content(), Some("The answer is 42."));
        assert_eq!(response.reasoning_content(), Some("Let me think..."));
        assert!(response.has_reasoning());
    }

    #[test]
    fn test_model_as_str() {
        assert_eq!(DeepSeekModel::DeepSeekChat.as_str(), "deepseek-chat");
        assert_eq!(DeepSeekModel::DeepSeekReasoner.as_str(), "deepseek-reasoner");
        assert_eq!(DeepSeekModel::DeepSeekCoder.as_str(), "deepseek-coder");
    }

    #[test]
    fn test_model_from_str() {
        assert_eq!(
            "deepseek-chat".parse::<DeepSeekModel>().unwrap(),
            DeepSeekModel::DeepSeekChat
        );
        assert_eq!(
            "deepseek-reasoner".parse::<DeepSeekModel>().unwrap(),
            DeepSeekModel::DeepSeekReasoner
        );
        assert_eq!(
            "deepseek-coder".parse::<DeepSeekModel>().unwrap(),
            DeepSeekModel::DeepSeekCoder
        );
    }

    #[test]
    fn test_model_max_context() {
        assert_eq!(DeepSeekModel::DeepSeekChat.max_context_tokens(), 64000);
        assert_eq!(DeepSeekModel::DeepSeekReasoner.max_context_tokens(), 64000);
    }

    #[test]
    fn test_model_is_reasoning() {
        assert!(DeepSeekModel::DeepSeekReasoner.is_reasoning());
        assert!(!DeepSeekModel::DeepSeekChat.is_reasoning());
        assert!(!DeepSeekModel::DeepSeekCoder.is_reasoning());
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
