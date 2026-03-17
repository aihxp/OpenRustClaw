//! Chat completions API.

use crate::client::OpenAIClient;
use crate::constants::endpoints;
use crate::error::{OpenAIError, Result};
use crate::types::{ChatMessage, ChatResponse, Function, Role, Tool};

/// Client for the chat completions API.
#[derive(Debug)]
pub struct Chat<'a> {
    client: &'a OpenAIClient,
}

impl<'a> Chat<'a> {
    /// Create a new chat client.
    pub fn new(client: &'a OpenAIClient) -> Self {
        Self { client }
    }

    /// Send a chat completion request.
    pub async fn complete(&self, request: ChatRequest) -> Result<ChatResponse> {
        let body = serde_json::to_value(&request)?;

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let body = body.clone();
                Box::pin(async move {
                    let response = client.post(endpoints::CHAT_COMPLETIONS, body).await?;
                    let body = client.handle_response(response).await?;
                    let chat_response: ChatResponse = serde_json::from_value(body)?;
                    Ok(chat_response)
                })
            })
            .await
    }

    /// Send a streaming chat completion request.
    #[cfg(feature = "streaming")]
    pub async fn stream(
        &self,
        mut request: ChatRequest,
    ) -> Result<impl futures::Stream<Item = Result<crate::streaming::ChatCompletionChunk>>> {
        use eventsource_stream::Eventsource;
        use futures::StreamExt;

        request.stream = Some(true);
        let body = serde_json::to_value(&request)?;

        let url = format!("{}{}", self.client.base_url(), endpoints::CHAT_COMPLETIONS);

        debug!("Initiating streaming chat completion request");

        let response = reqwest::Client::new()
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.client.api_key()))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(OpenAIError::from)?;

        let status = response.status();
        if !status.is_success() {
            return Err(OpenAIError::from_response(response).await);
        }

        let stream = response
            .bytes_stream()
            .eventsource()
            .map(|event| {
                match event {
                    Ok(event) => {
                        if event.data == "[DONE]" {
                            return Ok(crate::streaming::ChatCompletionChunk::done());
                        }

                        match serde_json::from_str::<crate::streaming::ChatCompletionChunk>(&event.data) {
                            Ok(chunk) => Ok(chunk),
                            Err(e) => Err(OpenAIError::Stream {
                                message: format!("Failed to parse SSE event: {e}"),
                            }),
                        }
                    }
                    Err(e) => Err(OpenAIError::Stream {
                        message: format!("SSE error: {e}"),
                    }),
                }
            });

        Ok(stream)
    }
}

/// A chat completion request.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChatRequest {
    /// ID of the model to use.
    pub model: String,

    /// The messages to generate chat completions for.
    pub messages: Vec<ChatMessage>,

    /// Maximum number of tokens to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<usize>,

    /// Temperature for sampling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Nucleus sampling parameter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,

    /// Number of completions to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<usize>,

    /// Whether to stream back partial progress.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    /// Stop sequences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,

    /// Presence penalty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,

    /// Frequency penalty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,

    /// Logit bias.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logit_bias: Option<serde_json::Value>,

    /// User identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,

    /// Tools available to the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,

    /// Tool choice strategy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,

    /// Response format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
}

/// Tool choice strategy.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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

/// Response format.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResponseFormat {
    /// The type of response format.
    #[serde(rename = "type")]
    pub format_type: String,
}

impl ResponseFormat {
    /// JSON object response format.
    pub fn json_object() -> Self {
        Self {
            format_type: "json_object".to_string(),
        }
    }

    /// Text response format.
    pub fn text() -> Self {
        Self {
            format_type: "text".to_string(),
        }
    }
}

impl ChatRequest {
    /// Create a new chat request builder.
    pub fn builder(model: impl Into<String>) -> ChatRequestBuilder {
        ChatRequestBuilder::new(model)
    }

    /// Create a simple chat request.
    pub fn simple(model: impl Into<String>, message: impl Into<String>) -> Self {
        Self::builder(model)
            .message(Role::User, message)
            .build()
    }

    /// Add a message to the conversation.
    pub fn add_message(&mut self, role: Role, content: impl Into<String>) {
        self.messages.push(ChatMessage::new(role, content));
    }

    /// Add a tool to the request.
    pub fn add_tool(&mut self, function: Function) {
        self.tools
            .get_or_insert_with(Vec::new)
            .push(Tool::function(function));
    }
}

/// Builder for chat requests.
#[derive(Debug, Clone)]
pub struct ChatRequestBuilder {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: Option<usize>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    n: Option<usize>,
    stream: Option<bool>,
    stop: Option<Vec<String>>,
    presence_penalty: Option<f32>,
    frequency_penalty: Option<f32>,
    user: Option<String>,
    tools: Option<Vec<Tool>>,
    tool_choice: Option<ToolChoice>,
    response_format: Option<ResponseFormat>,
}

impl ChatRequestBuilder {
    /// Create a new builder.
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            messages: Vec::new(),
            max_tokens: None,
            temperature: None,
            top_p: None,
            n: None,
            stream: None,
            stop: None,
            presence_penalty: None,
            frequency_penalty: None,
            user: None,
            tools: None,
            tool_choice: None,
            response_format: None,
        }
    }

    /// Add a message.
    pub fn message(mut self, role: Role, content: impl Into<String>) -> Self {
        self.messages.push(ChatMessage::new(role, content));
        self
    }

    /// Add a system message.
    pub fn system(mut self, content: impl Into<String>) -> Self {
        self.messages.push(ChatMessage::system(content));
        self
    }

    /// Add a user message.
    pub fn user(mut self, content: impl Into<String>) -> Self {
        self.messages.push(ChatMessage::user(content));
        self
    }

    /// Add an assistant message.
    pub fn assistant(mut self, content: impl Into<String>) -> Self {
        self.messages.push(ChatMessage::assistant(content));
        self
    }

    /// Set the messages.
    pub fn messages(mut self, messages: Vec<ChatMessage>) -> Self {
        self.messages = messages;
        self
    }

    /// Set max tokens.
    pub fn max_tokens(mut self, tokens: usize) -> Self {
        self.max_tokens = Some(tokens);
        self
    }

    /// Set temperature.
    pub fn temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp.clamp(0.0, 2.0));
        self
    }

    /// Set top-p.
    pub fn top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p.clamp(0.0, 1.0));
        self
    }

    /// Enable streaming.
    pub fn stream(mut self, enabled: bool) -> Self {
        self.stream = Some(enabled);
        self
    }

    /// Add a stop sequence.
    pub fn stop_sequence(mut self, seq: impl Into<String>) -> Self {
        self.stop.get_or_insert_with(Vec::new).push(seq.into());
        self
    }

    /// Set presence penalty.
    pub fn presence_penalty(mut self, penalty: f32) -> Self {
        self.presence_penalty = Some(penalty.clamp(-2.0, 2.0));
        self
    }

    /// Set frequency penalty.
    pub fn frequency_penalty(mut self, penalty: f32) -> Self {
        self.frequency_penalty = Some(penalty.clamp(-2.0, 2.0));
        self
    }

    /// Set user identifier.
    pub fn user_id(mut self, user: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self
    }

    /// Add a tool.
    pub fn tool(mut self, function: Function) -> Self {
        self.tools
            .get_or_insert_with(Vec::new)
            .push(Tool::function(function));
        self
    }

    /// Set tools.
    pub fn tools(mut self, tools: Vec<Tool>) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Set tool choice.
    pub fn tool_choice(mut self, choice: ToolChoice) -> Self {
        self.tool_choice = Some(choice);
        self
    }

    /// Set response format to JSON.
    pub fn json_mode(mut self) -> Self {
        self.response_format = Some(ResponseFormat::json_object());
        self
    }

    /// Build the request.
    pub fn build(self) -> ChatRequest {
        ChatRequest {
            model: self.model,
            messages: self.messages,
            max_tokens: self.max_tokens,
            temperature: self.temperature,
            top_p: self.top_p,
            n: self.n,
            stream: self.stream,
            stop: self.stop,
            presence_penalty: self.presence_penalty,
            frequency_penalty: self.frequency_penalty,
            logit_bias: None,
            user: self.user,
            tools: self.tools,
            tool_choice: self.tool_choice,
            response_format: self.response_format,
        }
    }
}

use tracing::debug;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_basic() {
        let request = ChatRequest::builder("gpt-4o")
            .max_tokens(1024)
            .user("Hello")
            .temperature(0.7)
            .build();

        assert_eq!(request.model, "gpt-4o");
        assert_eq!(request.max_tokens, Some(1024));
        assert_eq!(request.temperature, Some(0.7));
    }

    #[test]
    fn test_tool_choice() {
        let auto = ToolChoice::auto();
        let func = ToolChoice::function("get_weather");
        let none = ToolChoice::none();

        match func {
            ToolChoice::Specific { tool_type, function } => {
                assert_eq!(tool_type, "function");
                assert_eq!(function.name, "get_weather");
            }
            _ => panic!("Expected Specific tool choice"),
        }

        assert!(matches!(none, ToolChoice::Strategy(s) if s == "none"));
    }

    #[test]
    fn test_json_mode() {
        let request = ChatRequest::builder("gpt-4o")
            .user("Generate JSON")
            .json_mode()
            .build();

        assert!(request.response_format.is_some());
    }
}
