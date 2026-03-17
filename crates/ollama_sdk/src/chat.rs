//! Chat completions API for Ollama.

use crate::client::OllamaClient;
use crate::client::endpoints;
use crate::error::{OllamaError, Result};
use crate::types::{ChatMessage, ChatResponse, FormatType, ImageInput, KeepAlive, Options, Role, Tool};

/// Client for the chat API.
#[derive(Debug)]
pub struct Chat<'a> {
    client: &'a OllamaClient,
}

impl<'a> Chat<'a> {
    /// Create a new chat client.
    pub fn new(client: &'a OllamaClient) -> Self {
        Self { client }
    }

    /// Send a chat completion request.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ollama_sdk::{OllamaClient, ChatRequest, Role};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = OllamaClient::new("http://localhost:11434");
    ///
    /// let request = ChatRequest::builder("llama3.2")
    ///     .message(Role::User, "Hello!")
    ///     .build();
    ///
    /// let response = client.chat().generate(request).await?;
    /// println!("{}", response.message.content.unwrap_or_default());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn generate(&self, request: ChatRequest) -> Result<ChatResponse> {
        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let body = request.clone();
                Box::pin(async move {
                    let response = client.post(endpoints::CHAT, body).await?;
                    let chat_response: ChatResponse = client.handle_response(response).await?;
                    Ok(chat_response)
                })
            })
            .await
    }

    /// Send a streaming chat completion request.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ollama_sdk::{OllamaClient, ChatRequest, Role};
    /// use futures::StreamExt;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = OllamaClient::new("http://localhost:11434");
    ///
    /// let request = ChatRequest::builder("llama3.2")
    ///     .message(Role::User, "Tell me a story")
    ///     .build();
    ///
    /// let mut stream = client.chat().stream(request).await?;
    /// while let Some(chunk) = stream.next().await {
    ///     match chunk {
    ///         Ok(chunk) => print!("{}", chunk.message.content.unwrap_or_default()),
    ///         Err(e) => eprintln!("Error: {}", e),
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "streaming")]
    pub async fn stream(&self, mut request: ChatRequest) -> Result<ChatStream> {
        use futures::StreamExt;

        request.stream = Some(true);

        let url = format!("{}{}", self.client.base_url(), endpoints::CHAT);

        tracing::debug!("Initiating streaming chat completion request");

        let response = reqwest::Client::new()
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(OllamaError::from)?;

        let status = response.status();
        if !status.is_success() {
            return Err(OllamaError::from_response(response).await);
        }

        // Ollama uses NDJSON (newline-delimited JSON) for streaming
        let stream = response
            .bytes_stream()
            .map(|bytes| {
                match bytes {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);
                        let lines: Vec<&str> = text.lines().collect();
                        let results: Vec<Result<ChatStreamChunk>> = lines
                            .into_iter()
                            .filter(|line| !line.is_empty())
                            .map(|line| {
                                match serde_json::from_str::<ChatStreamChunk>(line) {
                                    Ok(chunk) => Ok(chunk),
                                    Err(e) => Err(OllamaError::Stream {
                                        message: format!("Failed to parse NDJSON: {e}"),
                                    }),
                                }
                            })
                            .collect();
                        
                        futures::stream::iter(results)
                    }
                    Err(e) => {
                        futures::stream::iter(vec![Err(OllamaError::Stream {
                            message: format!("Stream error: {e}"),
                        })])
                    }
                }
            })
            .flatten();

        Ok(ChatStream::new(stream))
    }
}

/// A chat completion request.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChatRequest {
    /// The model name.
    pub model: String,
    /// The messages in the conversation.
    pub messages: Vec<ChatMessage>,
    /// Tools available to the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    /// Format for the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<FormatType>,
    /// Additional model parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Options>,
    /// System prompt (alternative to system message).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// Template to use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    /// Context from previous generate requests (deprecated).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Vec<u64>>,
    /// Whether to stream the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    /// Controls how long the model stays loaded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_alive: Option<KeepAlive>,
}

impl ChatRequest {
    /// Create a new chat request builder.
    pub fn builder(model: impl Into<String>) -> ChatRequestBuilder {
        ChatRequestBuilder::new(model)
    }
}

/// Builder for chat requests.
#[derive(Debug, Clone)]
pub struct ChatRequestBuilder {
    model: String,
    messages: Vec<ChatMessage>,
    tools: Option<Vec<Tool>>,
    format: Option<FormatType>,
    options: Option<Options>,
    system: Option<String>,
    template: Option<String>,
    context: Option<Vec<u64>>,
    stream: Option<bool>,
    keep_alive: Option<KeepAlive>,
}

impl ChatRequestBuilder {
    /// Create a new builder.
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            messages: Vec::new(),
            tools: None,
            format: None,
            options: None,
            system: None,
            template: None,
            context: None,
            stream: None,
            keep_alive: None,
        }
    }

    /// Add a message.
    pub fn message(mut self, role: Role, content: impl Into<String>) -> Self {
        self.messages.push(ChatMessage::new(role, content));
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

    /// Add a system message.
    pub fn system(mut self, content: impl Into<String>) -> Self {
        self.messages.push(ChatMessage::system(content));
        self
    }

    /// Add a message with images for vision models.
    pub fn message_with_images(
        mut self,
        role: Role,
        content: impl Into<String>,
        images: Vec<ImageInput>,
    ) -> Self {
        // For simplicity, we'll encode synchronously here
        // In production, you might want to handle this differently
        let encoded_images: Vec<String> = images
            .into_iter()
            .filter_map(|img| img.to_base64().ok())
            .collect();

        self.messages.push(ChatMessage {
            role,
            content: Some(content.into()),
            tool_calls: None,
            images: Some(encoded_images),
        });
        self
    }

    /// Add a user message with images for vision models.
    pub fn user_with_images(
        self,
        content: impl Into<String>,
        images: Vec<ImageInput>,
    ) -> Self {
        self.message_with_images(Role::User, content, images)
    }

    /// Set the messages.
    pub fn messages(mut self, messages: Vec<ChatMessage>) -> Self {
        self.messages = messages;
        self
    }

    /// Add a tool.
    pub fn tool(mut self, tool: Tool) -> Self {
        self.tools.get_or_insert_with(Vec::new).push(tool);
        self
    }

    /// Set tools.
    pub fn tools(mut self, tools: Vec<Tool>) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Set the response format to JSON.
    pub fn json_mode(mut self) -> Self {
        self.format = Some(FormatType::json());
        self
    }

    /// Set the response format.
    pub fn format(mut self, format: FormatType) -> Self {
        self.format = Some(format);
        self
    }

    /// Set options.
    pub fn options(mut self, options: Options) -> Self {
        self.options = Some(options);
        self
    }

    /// Set temperature.
    pub fn temperature(mut self, temp: f32) -> Self {
        let opts = self.options.get_or_insert_with(Options::default);
        opts.temperature = Some(temp.clamp(0.0, 2.0));
        self
    }

    /// Set max tokens (num_predict).
    pub fn max_tokens(mut self, tokens: i32) -> Self {
        let opts = self.options.get_or_insert_with(Options::default);
        opts.num_predict = Some(tokens);
        self
    }

    /// Set the system prompt (alternative to system message).
    pub fn system_prompt(mut self, system: impl Into<String>) -> Self {
        self.system = Some(system.into());
        self
    }

    /// Set the template.
    pub fn template(mut self, template: impl Into<String>) -> Self {
        self.template = Some(template.into());
        self
    }

    /// Set the keep_alive duration.
    pub fn keep_alive(mut self, keep_alive: KeepAlive) -> Self {
        self.keep_alive = Some(keep_alive);
        self
    }

    /// Enable streaming.
    pub fn stream(mut self, enabled: bool) -> Self {
        self.stream = Some(enabled);
        self
    }

    /// Build the request.
    pub fn build(self) -> ChatRequest {
        ChatRequest {
            model: self.model,
            messages: self.messages,
            tools: self.tools,
            format: self.format,
            options: self.options,
            system: self.system,
            template: self.template,
            context: self.context,
            stream: self.stream,
            keep_alive: self.keep_alive,
        }
    }
}

/// A chat completion chunk in a stream.
#[cfg(feature = "streaming")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChatStreamChunk {
    /// The model used.
    pub model: String,
    /// The creation timestamp.
    pub created_at: String,
    /// The message delta.
    pub message: ChatMessage,
    /// Whether this is the final chunk.
    pub done: bool,
    /// Total duration in nanoseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_duration: Option<u64>,
    /// Load duration in nanoseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_duration: Option<u64>,
    /// Prompt evaluation count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_eval_count: Option<u64>,
    /// Prompt evaluation duration in nanoseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_eval_duration: Option<u64>,
    /// Evaluation count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_count: Option<u64>,
    /// Evaluation duration in nanoseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_duration: Option<u64>,
}

#[cfg(feature = "streaming")]
impl ChatStreamChunk {
    /// Check if this is the final chunk.
    pub fn is_done(&self) -> bool {
        self.done
    }

    /// Get the content of this chunk.
    pub fn content(&self) -> &str {
        self.message.content.as_deref().unwrap_or("")
    }
}

/// A streaming chat completion response.
#[cfg(feature = "streaming")]
pub struct ChatStream {
    inner: std::pin::Pin<Box<dyn futures::Stream<Item = Result<ChatStreamChunk>> + Send>>,
}

#[cfg(feature = "streaming")]
impl ChatStream {
    fn new<S>(stream: S) -> Self
    where
        S: futures::Stream<Item = Result<ChatStreamChunk>> + Send + 'static,
    {
        Self {
            inner: Box::pin(stream),
        }
    }
}

#[cfg(feature = "streaming")]
impl futures::Stream for ChatStream {
    type Item = Result<ChatStreamChunk>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}

#[cfg(feature = "streaming")]
impl std::fmt::Debug for ChatStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChatStream").finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Function;

    #[test]
    fn test_builder_basic() {
        let request = ChatRequest::builder("llama3.2")
            .user("Hello")
            .assistant("Hi there!")
            .build();

        assert_eq!(request.model, "llama3.2");
        assert_eq!(request.messages.len(), 2);
    }

    #[test]
    fn test_builder_with_options() {
        let request = ChatRequest::builder("llama3.2")
            .user("Hello")
            .temperature(0.7)
            .max_tokens(1024)
            .json_mode()
            .build();

        assert!(request.format.is_some());
        assert!(request.options.is_some());
        assert_eq!(request.options.unwrap().temperature, Some(0.7));
    }

    #[test]
    fn test_builder_with_tools() {
        let tool = Tool::function(
            Function::builder("get_weather", "Get weather")
                .string_property("city", "City name", true)
                .build(),
        );

        let request = ChatRequest::builder("llama3.2")
            .user("What's the weather?")
            .tool(tool)
            .build();

        assert!(request.tools.is_some());
        assert_eq!(request.tools.unwrap().len(), 1);
    }

    #[test]
    fn test_keep_alive() {
        let request = ChatRequest::builder("llama3.2")
            .user("Hello")
            .keep_alive(KeepAlive::indefinitely())
            .build();

        assert!(matches!(request.keep_alive, Some(KeepAlive::Seconds(-1))));
    }
}
