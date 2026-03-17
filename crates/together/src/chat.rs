//! Chat completions API for Together AI.

use crate::client::TogetherClient;
use crate::constants::endpoints;
use crate::error::Result;
use crate::types::{ChatMessage, ChatResponse, ResponseFormat, Role, Tool};

/// Client for chat completions.
#[derive(Debug)]
pub struct Chat<'a> {
    client: &'a TogetherClient,
}

impl<'a> Chat<'a> {
    /// Create a new chat client.
    pub fn new(client: &'a TogetherClient) -> Self {
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
    pub async fn complete_stream(
        &self,
        request: ChatRequest,
    ) -> Result<crate::streaming::ChatCompletionStream> {
        use crate::streaming::ChatCompletionStream;
        
        let mut request = request;
        request.stream = Some(true);
        
        let body = serde_json::to_value(&request)?;
        
        let response = self
            .client
            .post(endpoints::CHAT_COMPLETIONS, body)
            .await?;
        
        if !response.status().is_success() {
            return Err(crate::error::TogetherError::from_response(response).await);
        }
        
        Ok(ChatCompletionStream::new(response))
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
    pub tool_choice: Option<String>,
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
    /// Number of logprobs to return (0 to 5).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_logprobs: Option<usize>,
    /// Number of completions to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<usize>,
    /// Response format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
    /// Safety model for moderation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_model: Option<String>,
    /// Random seed for reproducibility.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
}

impl ChatRequest {
    /// Create a new builder.
    pub fn builder(model: impl Into<String>) -> ChatRequestBuilder {
        ChatRequestBuilder::new(model)
    }

    /// Create a simple request.
    pub fn simple(model: impl Into<String>, message: impl Into<String>) -> Self {
        Self::builder(model)
            .message(Role::User, message)
            .build()
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
    tool_choice: Option<String>,
    presence_penalty: Option<f32>,
    frequency_penalty: Option<f32>,
    logit_bias: Option<crate::types::LogitBias>,
    logprobs: Option<bool>,
    top_logprobs: Option<usize>,
    n: Option<usize>,
    response_format: Option<ResponseFormat>,
    safety_model: Option<String>,
    seed: Option<i64>,
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
            safety_model: None,
            seed: None,
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
    pub fn tool_choice(mut self, choice: impl Into<String>) -> Self {
        self.tool_choice = Some(choice.into());
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
        self.top_logprobs = Some(n.min(5));
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

    /// Set safety model.
    pub fn safety_model(mut self, model: impl Into<String>) -> Self {
        self.safety_model = Some(model.into());
        self
    }

    /// Set random seed.
    pub fn seed(mut self, seed: i64) -> Self {
        self.seed = Some(seed);
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
            safety_model: self.safety_model,
            seed: self.seed,
        }
    }
}
