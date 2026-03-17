//! Chat completion types for the Azure OpenAI API.

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
    /// Function message (deprecated).
    Function,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::System => write!(f, "system"),
            Role::User => write!(f, "user"),
            Role::Assistant => write!(f, "assistant"),
            Role::Tool => write!(f, "tool"),
            Role::Function => write!(f, "function"),
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
    /// The name of the author (for function/tool messages).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Tool calls made by the assistant.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    /// The tool call ID this message is responding to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// Function call (deprecated).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<FunctionCall>,
    /// Context for the message (Azure-specific).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<serde_json::Value>,
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
            function_call: None,
            context: None,
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
            function_call: None,
            context: None,
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
            function_call: None,
            context: None,
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
    pub tool_type: ToolType,
    /// The function definition.
    pub function: Function,
}

/// Tool type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolType {
    /// Function tool.
    Function,
}

impl Tool {
    /// Create a new tool from a function definition.
    pub fn function(function: Function) -> Self {
        Self {
            tool_type: ToolType::Function,
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
    /// Whether to enable strict mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

impl Function {
    /// Create a new function definition.
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: Some(description.into()),
            parameters: serde_json::json!({"type": "object"}),
            strict: None,
        }
    }

    /// Set the parameters schema.
    pub fn parameters(mut self, params: serde_json::Value) -> Self {
        self.parameters = params;
        self
    }

    /// Enable strict mode.
    pub fn strict(mut self, strict: bool) -> Self {
        self.strict = Some(strict);
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
    strict: Option<bool>,
}

impl FunctionBuilder {
    /// Create a new function builder.
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            properties: Vec::new(),
            required: Vec::new(),
            strict: None,
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

    /// Enable strict mode.
    pub fn strict(mut self, strict: bool) -> Self {
        self.strict = Some(strict);
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
            strict: self.strict,
        }
    }
}

/// Tool choice strategy.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolChoice {
    /// Strategy string.
    Strategy(String),
    /// Specific tool.
    Specific {
        /// Type (always "function").
        #[serde(rename = "type")]
        tool_type: String,
        /// Function specification.
        function: ToolChoiceFunction,
    },
}

/// Specific tool choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolChoiceFunction {
    /// The name of the function to call.
    pub name: String,
}

impl ToolChoice {
    /// Auto tool choice.
    pub fn auto() -> Self {
        ToolChoice::Strategy("auto".to_string())
    }

    /// None tool choice.
    pub fn none() -> Self {
        ToolChoice::Strategy("none".to_string())
    }

    /// Required tool choice.
    pub fn required() -> Self {
        ToolChoice::Strategy("required".to_string())
    }

    /// Specific tool choice.
    pub fn function(name: impl Into<String>) -> Self {
        ToolChoice::Specific {
            tool_type: "function".to_string(),
            function: ToolChoiceFunction { name: name.into() },
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
    pub usage: super::TokenUsage,
    /// System fingerprint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
    /// Content filter results for the prompt.
    #[serde(
        rename = "prompt_filter_results",
        skip_serializing_if = "Option::is_none"
    )]
    pub prompt_filter_results: Option<Vec<super::PromptFilterResult>>,
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
    pub fn finish_reason(&self) -> Option<super::FinishReason> {
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
    pub finish_reason: Option<super::FinishReason>,
    /// Content filter results.
    #[serde(
        rename = "content_filter_results",
        skip_serializing_if = "Option::is_none"
    )]
    pub content_filter_results: Option<super::ContentFilterResults>,
    /// Log probabilities (if requested).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{FinishReason, TokenUsage};

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
            model: "gpt-4".to_string(),
            choices: vec![ChatChoice {
                index: 0,
                message: ChatMessage::assistant("Hello!"),
                finish_reason: Some(FinishReason::Stop),
                content_filter_results: None,
                logprobs: None,
            }],
            usage: TokenUsage::new(10, 5),
            system_fingerprint: None,
            prompt_filter_results: None,
        };

        assert_eq!(response.content(), Some("Hello!"));
        assert!(!response.has_tool_calls());
    }

    #[test]
    fn test_tool_choice() {
        let auto = ToolChoice::auto();
        let func = ToolChoice::function("get_weather");
        let none = ToolChoice::none();

        match func {
            ToolChoice::Specific {
                tool_type,
                function,
            } => {
                assert_eq!(tool_type, "function");
                assert_eq!(function.name, "get_weather");
            }
            _ => panic!("Expected Specific tool choice"),
        }

        assert!(matches!(none, ToolChoice::Strategy(s) if s == "none"));
    }
}
