//! Request types for the Anthropic Messages API.

use super::{Message, SystemContent, Tool};
use serde::{Deserialize, Serialize};

/// A request to the Anthropic Messages API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRequest {
    /// The model to use (e.g., "claude-3-5-sonnet-20241022").
    pub model: String,

    /// The maximum number of tokens to generate.
    pub max_tokens: usize,

    /// The messages in the conversation.
    pub messages: Vec<Message>,

    /// System prompt(s).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<SystemContent>,

    /// Tools available to the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,

    /// How the model should use tools.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,

    /// Whether to stream the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    /// Temperature for sampling (0.0 to 1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Top-p sampling parameter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,

    /// Top-k sampling parameter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<usize>,

    /// Stop sequences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,

    /// Metadata about the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,


}

impl MessageRequest {
    /// Create a new request builder for the given model.
    pub fn builder(model: impl Into<String>) -> MessageRequestBuilder {
        MessageRequestBuilder::new(model)
    }

    /// Create a simple request with a single user message.
    pub fn simple(model: impl Into<String>, message: impl Into<String>) -> Self {
        Self::builder(model).message(super::MessageRole::User, message).build()
    }

    /// Add a message to this request.
    pub fn add_message(&mut self, role: super::MessageRole, text: impl Into<String>) {
        self.messages.push(Message::new(role, text));
    }

    /// Add a tool to this request.
    pub fn add_tool(&mut self, tool: Tool) {
        self.tools.get_or_insert_with(Vec::new).push(tool);
    }

    /// Enable streaming for this request.
    pub fn with_streaming(mut self, enabled: bool) -> Self {
        self.stream = Some(enabled);
        self
    }

    /// Get the estimated token count for this request (rough approximation).
    pub fn estimated_tokens(&self) -> usize {
        // Rough estimation: 4 chars ~= 1 token for English text
        let text: String = self
            .messages
            .iter()
            .map(|m| m.text())
            .collect::<Vec<_>>()
            .join(" ");
        text.len() / 4
    }
}

/// Builder for message requests.
#[derive(Debug, Clone)]
pub struct MessageRequestBuilder {
    model: String,
    max_tokens: usize,
    messages: Vec<Message>,
    system: Option<SystemContent>,
    tools: Option<Vec<Tool>>,
    tool_choice: Option<ToolChoice>,
    stream: Option<bool>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    top_k: Option<usize>,
    stop_sequences: Option<Vec<String>>,
    metadata: Option<Metadata>,
}

impl MessageRequestBuilder {
    /// Create a new builder for the given model.
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            max_tokens: 4096,
            messages: Vec::new(),
            system: None,
            tools: None,
            tool_choice: None,
            stream: None,
            temperature: None,
            top_p: None,
            top_k: None,
            stop_sequences: None,
            metadata: None,
        }
    }

    /// Set the maximum tokens to generate.
    pub fn max_tokens(mut self, tokens: usize) -> Self {
        self.max_tokens = tokens;
        self
    }

    /// Add a user message.
    pub fn message(mut self, role: super::MessageRole, text: impl Into<String>) -> Self {
        self.messages.push(Message::new(role, text));
        self
    }

    /// Add a user message (convenience method).
    pub fn user(mut self, text: impl Into<String>) -> Self {
        self.messages.push(Message::user(text));
        self
    }

    /// Add an assistant message (convenience method).
    pub fn assistant(mut self, text: impl Into<String>) -> Self {
        self.messages.push(Message::assistant(text));
        self
    }

    /// Set the system prompt.
    pub fn system(mut self, prompt: impl Into<String>) -> Self {
        self.system = Some(SystemContent::Text(prompt.into()));
        self
    }

    /// Set the system prompt with structured blocks.
    pub fn system_blocks(mut self, blocks: Vec<super::SystemBlock>) -> Self {
        self.system = Some(SystemContent::Blocks(blocks));
        self
    }

    /// Add a tool.
    pub fn tool(mut self, tool: Tool) -> Self {
        self.tools.get_or_insert_with(Vec::new).push(tool);
        self
    }

    /// Set the tools.
    pub fn tools(mut self, tools: Vec<Tool>) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Set the tool choice.
    pub fn tool_choice(mut self, choice: ToolChoice) -> Self {
        self.tool_choice = Some(choice);
        self
    }

    /// Enable/disable streaming.
    pub fn stream(mut self, enabled: bool) -> Self {
        self.stream = Some(enabled);
        self
    }

    /// Set the temperature.
    pub fn temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp.clamp(0.0, 1.0));
        self
    }

    /// Set the top-p parameter.
    pub fn top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p.clamp(0.0, 1.0));
        self
    }

    /// Set the top-k parameter.
    pub fn top_k(mut self, top_k: usize) -> Self {
        self.top_k = Some(top_k);
        self
    }

    /// Add a stop sequence.
    pub fn stop_sequence(mut self, seq: impl Into<String>) -> Self {
        self.stop_sequences
            .get_or_insert_with(Vec::new)
            .push(seq.into());
        self
    }

    /// Set the stop sequences.
    pub fn stop_sequences(mut self, seqs: Vec<String>) -> Self {
        self.stop_sequences = Some(seqs);
        self
    }

    /// Set metadata.
    pub fn metadata(mut self, metadata: Metadata) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Build the request.
    pub fn build(self) -> MessageRequest {
        MessageRequest {
            model: self.model,
            max_tokens: self.max_tokens,
            messages: self.messages,
            system: self.system,
            tools: self.tools,
            tool_choice: self.tool_choice,
            stream: self.stream,
            temperature: self.temperature,
            top_p: self.top_p,
            top_k: self.top_k,
            stop_sequences: self.stop_sequences,
            metadata: self.metadata,

        }
    }
}

/// Metadata about the request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    /// An external identifier for the user making the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

impl Metadata {
    /// Create metadata with a user ID.
    pub fn user_id(id: impl Into<String>) -> Self {
        Self {
            user_id: Some(id.into()),
        }
    }
}

/// Tool choice options.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolChoice {
    /// Let the model decide whether to use tools.
    Auto,
    /// Force the model to use any tool.
    Any,
    /// Force the model to use a specific tool.
    Tool {
        /// The name of the tool to use.
        name: String,
    },
    /// Prevent the model from using tools.
    None,
}

impl Default for ToolChoice {
    fn default() -> Self {
        ToolChoice::Auto
    }
}

impl ToolChoice {
    /// Auto tool choice.
    pub fn auto() -> Self {
        ToolChoice::Auto
    }

    /// Any tool choice.
    pub fn any() -> Self {
        ToolChoice::Any
    }

    /// Specific tool choice.
    pub fn tool(name: impl Into<String>) -> Self {
        ToolChoice::Tool { name: name.into() }
    }

    /// None tool choice.
    pub fn none() -> Self {
        ToolChoice::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_basic() {
        let request = MessageRequest::builder("claude-3-sonnet")
            .max_tokens(1024)
            .user("Hello")
            .temperature(0.7)
            .build();

        assert_eq!(request.model, "claude-3-sonnet");
        assert_eq!(request.max_tokens, 1024);
        assert_eq!(request.messages.len(), 1);
        assert_eq!(request.temperature, Some(0.7));
    }

    #[test]
    fn test_builder_chain() {
        let request = MessageRequest::builder("claude-3-opus")
            .system("You are helpful.")
            .user("Hello")
            .assistant("Hi there!")
            .max_tokens(500)
            .stream(true)
            .build();

        assert_eq!(request.messages.len(), 2);
        assert_eq!(request.stream, Some(true));
        assert!(request.system.is_some());
    }

    #[test]
    fn test_simple_request() {
        let request = MessageRequest::simple("claude-3-haiku", "Hello");
        assert_eq!(request.model, "claude-3-haiku");
        assert_eq!(request.messages.len(), 1);
    }

    #[test]
    fn test_tool_choice_serialization() {
        let auto = ToolChoice::auto();
        let json = serde_json::to_string(&auto).unwrap();
        assert!(json.contains("auto"));

        let tool = ToolChoice::tool("get_weather");
        let json = serde_json::to_string(&tool).unwrap();
        assert!(json.contains("tool"));
        assert!(json.contains("get_weather"));
    }

    #[test]
    fn test_estimated_tokens() {
        let request = MessageRequest::simple("claude-3-haiku", "Hello world this is a test");
        // Roughly 26 chars / 4 = ~6.5 tokens
        assert!(request.estimated_tokens() > 0);
    }
}
