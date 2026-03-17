//! Chat completions API for vLLM.

use crate::client::VllmClient;
use crate::constants::endpoints;
use crate::error::Result;
use crate::types::{ChatMessage, ChatResponse, ResponseFormat, Role, Tool, ToolChoice};

/// Client for chat completions.
#[derive(Debug)]
pub struct Chat<'a> {
    client: &'a VllmClient,
}

impl<'a> Chat<'a> {
    /// Create a new chat client.
    pub fn new(client: &'a VllmClient) -> Self {
        Self { client }
    }

    /// Send a chat completion request.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use vllm::{VllmClient, ChatRequest, Role};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = VllmClient::new("http://localhost:8000")?;
    ///
    /// let request = ChatRequest::builder("meta-llama/Llama-3-8b-chat-hf")
    ///     .message(Role::User, "Hello, vLLM!")
    ///     .build();
    ///
    /// let response = client.chat().complete(request).await?;
    /// println!("{}", response.content());
    /// # Ok(())
    /// # }
    /// ```
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
    ///
    /// # Example
    ///
    /// ```no_run
    /// use vllm::{VllmClient, ChatRequest, Role};
    /// use futures::StreamExt;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = VllmClient::new("http://localhost:8000")?;
    ///
    /// let request = ChatRequest::builder("meta-llama/Llama-3-8b-chat-hf")
    ///     .message(Role::User, "Tell me a story")
    ///     .build();
    ///
    /// let mut stream = client.chat().complete_stream(request).await?;
    /// while let Some(chunk) = stream.next().await {
    ///     match chunk {
    ///         Ok(chunk) => print!("{}", chunk.content()),
    ///         Err(e) => eprintln!("Error: {}", e),
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "streaming")]
    pub async fn complete_stream(
        &self,
        request: ChatRequest,
    ) -> Result<crate::streaming::ChatStream> {
        use crate::streaming::ChatStream;

        let mut request = request;
        request.stream = Some(true);

        let body = serde_json::to_value(&request)?;

        let response = self.client.post(endpoints::CHAT_COMPLETIONS, body).await?;

        if !response.status().is_success() {
            return Err(crate::error::VllmError::from_response(response).await);
        }

        Ok(ChatStream::new(response))
    }
}

/// A chat completion request.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChatRequest {
    /// Model ID.
    pub model: String,
    /// Messages.
    pub messages: Vec<ChatMessage>,
    /// Max tokens to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<usize>,
    /// Temperature (0.0 to 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Top-p sampling (0.0 to 1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Top-k sampling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<usize>,
    /// Whether to stream.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    /// Stop sequences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    /// Tools.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    /// Tool choice.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    /// Presence penalty (-2.0 to 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    /// Frequency penalty (-2.0 to 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    /// Logit bias.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logit_bias: Option<crate::types::LogitBias>,
    /// Whether to return logprobs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<bool>,
    /// Number of logprobs to return (0 to 20).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_logprobs: Option<usize>,
    /// Number of completions to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<usize>,
    /// Response format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
    /// Random seed for reproducibility.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
    /// User identifier for tracking.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    /// Best of - generate N candidates and return the best.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best_of: Option<usize>,
    /// Use beam search.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_beam_search: Option<bool>,
    /// Early stopping.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub early_stopping: Option<bool>,
    /// Ignore EOS token.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore_eos: Option<bool>,
}

impl ChatRequest {
    /// Create a new builder.
    pub fn builder(model: impl Into<String>) -> ChatRequestBuilder {
        ChatRequestBuilder::new(model)
    }

    /// Create a simple request.
    pub fn simple(model: impl Into<String>, message: impl Into<String>) -> Self {
        Self::builder(model).message(Role::User, message).build()
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
    top_k: Option<usize>,
    stream: Option<bool>,
    stop: Option<Vec<String>>,
    tools: Option<Vec<Tool>>,
    tool_choice: Option<ToolChoice>,
    presence_penalty: Option<f32>,
    frequency_penalty: Option<f32>,
    logit_bias: Option<crate::types::LogitBias>,
    logprobs: Option<bool>,
    top_logprobs: Option<usize>,
    n: Option<usize>,
    response_format: Option<ResponseFormat>,
    seed: Option<i64>,
    user: Option<String>,
    best_of: Option<usize>,
    use_beam_search: Option<bool>,
    early_stopping: Option<bool>,
    ignore_eos: Option<bool>,
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
            top_k: None,
            stream: None,
            stop: None,
            tools: None,
            tool_choice: None,
            presence_penalty: None,
            frequency_penalty: None,
            logit_bias: None,
            logprobs: None,
            top_logprobs: None,
            n: None,
            response_format: None,
            seed: None,
            user: None,
            best_of: None,
            use_beam_search: None,
            early_stopping: None,
            ignore_eos: None,
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

    /// Set top-k.
    pub fn top_k(mut self, top_k: usize) -> Self {
        self.top_k = Some(top_k);
        self
    }

    /// Enable streaming.
    pub fn stream(mut self, enabled: bool) -> Self {
        self.stream = Some(enabled);
        self
    }

    /// Add a stop sequence.
    pub fn stop(mut self, stop: impl Into<String>) -> Self {
        self.stop.get_or_insert_with(Vec::new).push(stop.into());
        self
    }

    /// Add a tool.
    pub fn tool(mut self, tool: Tool) -> Self {
        self.tools.get_or_insert_with(Vec::new).push(tool);
        self
    }

    /// Set tool choice.
    pub fn tool_choice(mut self, choice: ToolChoice) -> Self {
        self.tool_choice = Some(choice);
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

    /// Set logprobs.
    pub fn logprobs(mut self, enabled: bool) -> Self {
        self.logprobs = Some(enabled);
        self
    }

    /// Set top logprobs.
    pub fn top_logprobs(mut self, n: usize) -> Self {
        self.top_logprobs = Some(n.min(20));
        self
    }

    /// Set number of completions.
    pub fn n(mut self, n: usize) -> Self {
        self.n = Some(n);
        self
    }

    /// Set response format.
    pub fn response_format(mut self, format: ResponseFormat) -> Self {
        self.response_format = Some(format);
        self
    }

    /// Set JSON mode (shortcut for JSON response format).
    pub fn json_mode(mut self) -> Self {
        self.response_format = Some(ResponseFormat::JsonObject);
        self
    }

    /// Set random seed.
    pub fn seed(mut self, seed: i64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Set user identifier for tracking.
    pub fn user_id(mut self, user: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self
    }

    /// Set best of.
    pub fn best_of(mut self, best_of: usize) -> Self {
        self.best_of = Some(best_of);
        self
    }

    /// Enable beam search.
    pub fn use_beam_search(mut self, enabled: bool) -> Self {
        self.use_beam_search = Some(enabled);
        self
    }

    /// Set early stopping.
    pub fn early_stopping(mut self, enabled: bool) -> Self {
        self.early_stopping = Some(enabled);
        self
    }

    /// Ignore EOS token.
    pub fn ignore_eos(mut self, enabled: bool) -> Self {
        self.ignore_eos = Some(enabled);
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
            top_k: self.top_k,
            stream: self.stream,
            stop: self.stop,
            tools: self.tools,
            tool_choice: self.tool_choice,
            presence_penalty: self.presence_penalty,
            frequency_penalty: self.frequency_penalty,
            logit_bias: self.logit_bias,
            logprobs: self.logprobs,
            top_logprobs: self.top_logprobs,
            n: self.n,
            response_format: self.response_format,
            seed: self.seed,
            user: self.user,
            best_of: self.best_of,
            use_beam_search: self.use_beam_search,
            early_stopping: self.early_stopping,
            ignore_eos: self.ignore_eos,
        }
    }
}
