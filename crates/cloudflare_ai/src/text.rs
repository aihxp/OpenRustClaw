//! Text generation API for Cloudflare Workers AI.

use crate::client::CloudflareAiClient;
use crate::error::{CloudflareAiError, Result};
use crate::types::{ChatMessage, TextGenerationResponse, TextGenerationStreamResponse};

/// Client for the text generation API.
#[derive(Debug)]
pub struct Text<'a> {
    client: &'a CloudflareAiClient,
}

impl<'a> Text<'a> {
    /// Create a new text generation client.
    pub fn new(client: &'a CloudflareAiClient) -> Self {
        Self { client }
    }

    /// Generate text using a model.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use cloudflare_ai::{CloudflareAiClient, TextGenerationRequest};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = CloudflareAiClient::new("your-account-id", "your-api-token")?;
    ///
    /// let request = TextGenerationRequest::builder("@cf/meta/llama-3-8b-instruct")
    ///     .prompt("What is the capital of France?")
    ///     .build();
    ///
    /// let response = client.text().generate(request).await?;
    /// println!("{}", response.text());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn generate(&self, request: TextGenerationRequest) -> Result<TextGenerationResponse> {
        let model = request.model.clone();
        let body = serde_json::to_value(&request)?;

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let body = body.clone();
                let model = model.clone();
                Box::pin(async move {
                    let response = client.post(&model, body).await?;
                    let result: TextGenerationResponse = client.handle_response(response).await?;
                    Ok(result)
                })
            })
            .await
    }

    /// Generate text using messages (chat format).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use cloudflare_ai::{CloudflareAiClient, ChatMessage, Role};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = CloudflareAiClient::new("your-account-id", "your-api-token")?;
    ///
    /// let messages = vec![
    ///     ChatMessage::system("You are a helpful assistant."),
    ///     ChatMessage::user("What is the capital of France?"),
    /// ];
    ///
    /// let response = client.text()
    ///     .chat("@cf/meta/llama-3-8b-instruct", messages)
    ///     .await?;
    /// println!("{}", response.text());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn chat(
        &self,
        model: impl Into<String>,
        messages: Vec<ChatMessage>,
    ) -> Result<TextGenerationResponse> {
        let request = TextGenerationRequest {
            model: model.into(),
            prompt: None,
            messages: Some(messages),
            raw: None,
            stream: None,
            max_tokens: None,
            temperature: None,
            top_p: None,
            top_k: None,
            seed: None,
            repetition_penalty: None,
            frequency_penalty: None,
            presence_penalty: None,
        };
        self.generate(request).await
    }

    /// Generate text using a simple prompt.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use cloudflare_ai::CloudflareAiClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = CloudflareAiClient::new("your-account-id", "your-api-token")?;
    ///
    /// let response = client.text()
    ///     .prompt("@cf/meta/llama-3-8b-instruct", "What is the capital of France?")
    ///     .await?;
    /// println!("{}", response.text());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn prompt(
        &self,
        model: impl Into<String>,
        prompt: impl Into<String>,
    ) -> Result<TextGenerationResponse> {
        let request = TextGenerationRequest::builder(model).prompt(prompt).build();
        self.generate(request).await
    }

    /// Send a streaming text generation request.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use cloudflare_ai::{CloudflareAiClient, TextGenerationRequest};
    /// use futures::StreamExt;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = CloudflareAiClient::new("your-account-id", "your-api-token")?;
    ///
    /// let request = TextGenerationRequest::builder("@cf/meta/llama-3-8b-instruct")
    ///     .prompt("Tell me a story")
    ///     .build();
    ///
    /// let text_client = client.text();
    /// let mut stream = text_client.stream(request).await?;
    /// while let Some(chunk) = stream.next().await {
    ///     match chunk {
    ///         Ok(chunk) => print!("{}", chunk.text()),
    ///         Err(e) => eprintln!("Error: {}", e),
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "streaming")]
    pub async fn stream(
        &self,
        mut request: TextGenerationRequest,
    ) -> Result<impl futures::Stream<Item = Result<TextGenerationStreamResponse>>> {
        use eventsource_stream::Eventsource;
        use futures::StreamExt;

        request.stream = Some(true);
        let model = request.model.clone();
        let body = serde_json::to_value(&request)?;
        let url = self.client.build_url(&model);

        tracing::debug!("Initiating streaming text generation request");

        let response = reqwest::Client::new()
            .post(&url)
            .header(
                "Authorization",
                format!("Bearer {}", self.client.account_id()),
            )
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(CloudflareAiError::from)?;

        let status = response.status();
        if !status.is_success() {
            return Err(CloudflareAiError::from_response(response).await);
        }

        let stream = response
            .bytes_stream()
            .eventsource()
            .map(|event| match event {
                Ok(event) => {
                    if event.data == "[DONE]" {
                        return Ok(TextGenerationStreamResponse {
                            response: String::new(),
                        });
                    }

                    match serde_json::from_str::<TextGenerationStreamResponse>(&event.data) {
                        Ok(chunk) => Ok(chunk),
                        Err(e) => Err(CloudflareAiError::Stream {
                            message: format!("Failed to parse SSE event: {e}"),
                        }),
                    }
                }
                Err(e) => Err(CloudflareAiError::Stream {
                    message: format!("SSE error: {e}"),
                }),
            })
            .filter(|chunk| {
                // Filter out empty chunks
                futures::future::ready(!matches!(chunk, Ok(c) if c.is_empty()))
            });

        Ok(stream)
    }
}

/// A text generation request.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TextGenerationRequest {
    /// ID of the model to use (e.g., "@cf/meta/llama-3-8b-instruct").
    #[serde(skip_serializing)]
    pub model: String,
    /// The prompt for the model (for simple text generation).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// The messages for chat-based models.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub messages: Option<Vec<ChatMessage>>,
    /// Whether to use raw mode (bypass special token handling).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<bool>,
    /// Whether to stream back partial progress.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    /// Maximum number of tokens to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<usize>,
    /// Temperature for sampling (0.0 to 5.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Top-p sampling parameter (0.0 to 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Top-k sampling parameter (1 to 50).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<usize>,
    /// Random seed for reproducibility.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
    /// Repetition penalty (1.0 to 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repetition_penalty: Option<f32>,
    /// Frequency penalty (-2.0 to 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    /// Presence penalty (-2.0 to 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
}

impl TextGenerationRequest {
    /// Create a new text generation request builder.
    pub fn builder(model: impl Into<String>) -> TextGenerationRequestBuilder {
        TextGenerationRequestBuilder::new(model)
    }

    /// Create a simple text generation request with a prompt.
    pub fn prompt(model: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self::builder(model).prompt(prompt).build()
    }

    /// Create a chat-based text generation request.
    pub fn chat(model: impl Into<String>, messages: Vec<ChatMessage>) -> Self {
        Self::builder(model).messages(messages).build()
    }

    /// Add a message to the conversation.
    pub fn add_message(&mut self, role: crate::types::Role, content: impl Into<String>) {
        if self.messages.is_none() {
            self.messages = Some(Vec::new());
        }
        if let Some(ref mut messages) = self.messages {
            messages.push(ChatMessage::new(role, content));
        }
    }
}

/// Builder for text generation requests.
#[derive(Debug, Clone)]
pub struct TextGenerationRequestBuilder {
    model: String,
    prompt: Option<String>,
    messages: Option<Vec<ChatMessage>>,
    raw: Option<bool>,
    stream: Option<bool>,
    max_tokens: Option<usize>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    top_k: Option<usize>,
    seed: Option<i64>,
    repetition_penalty: Option<f32>,
    frequency_penalty: Option<f32>,
    presence_penalty: Option<f32>,
}

impl TextGenerationRequestBuilder {
    /// Create a new builder.
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            prompt: None,
            messages: None,
            raw: None,
            stream: None,
            max_tokens: None,
            temperature: None,
            top_p: None,
            top_k: None,
            seed: None,
            repetition_penalty: None,
            frequency_penalty: None,
            presence_penalty: None,
        }
    }

    /// Set the prompt (for simple text generation).
    pub fn prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = Some(prompt.into());
        self.messages = None; // Mutually exclusive
        self
    }

    /// Set the messages (for chat-based generation).
    pub fn messages(mut self, messages: Vec<ChatMessage>) -> Self {
        self.messages = Some(messages);
        self.prompt = None; // Mutually exclusive
        self
    }

    /// Add a single message.
    pub fn message(mut self, role: crate::types::Role, content: impl Into<String>) -> Self {
        self.messages
            .get_or_insert_with(Vec::new)
            .push(ChatMessage::new(role, content));
        self.prompt = None;
        self
    }

    /// Add a system message.
    pub fn system(self, content: impl Into<String>) -> Self {
        self.message(crate::types::Role::System, content)
    }

    /// Add a user message.
    pub fn user(self, content: impl Into<String>) -> Self {
        self.message(crate::types::Role::User, content)
    }

    /// Add an assistant message.
    pub fn assistant(self, content: impl Into<String>) -> Self {
        self.message(crate::types::Role::Assistant, content)
    }

    /// Enable raw mode (bypass special token handling).
    pub fn raw(mut self, enabled: bool) -> Self {
        self.raw = Some(enabled);
        self
    }

    /// Enable streaming.
    pub fn stream(mut self, enabled: bool) -> Self {
        self.stream = Some(enabled);
        self
    }

    /// Set max tokens.
    pub fn max_tokens(mut self, tokens: usize) -> Self {
        self.max_tokens = Some(tokens);
        self
    }

    /// Set temperature (0.0 to 5.0).
    pub fn temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp.clamp(0.0, 5.0));
        self
    }

    /// Set top-p (0.0 to 2.0).
    pub fn top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p.clamp(0.0, 2.0));
        self
    }

    /// Set top-k (1 to 50).
    pub fn top_k(mut self, top_k: usize) -> Self {
        self.top_k = Some(top_k.clamp(1, 50));
        self
    }

    /// Set random seed.
    pub fn seed(mut self, seed: i64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Set repetition penalty (1.0 to 2.0).
    pub fn repetition_penalty(mut self, penalty: f32) -> Self {
        self.repetition_penalty = Some(penalty.clamp(1.0, 2.0));
        self
    }

    /// Set frequency penalty (-2.0 to 2.0).
    pub fn frequency_penalty(mut self, penalty: f32) -> Self {
        self.frequency_penalty = Some(penalty.clamp(-2.0, 2.0));
        self
    }

    /// Set presence penalty (-2.0 to 2.0).
    pub fn presence_penalty(mut self, penalty: f32) -> Self {
        self.presence_penalty = Some(penalty.clamp(-2.0, 2.0));
        self
    }

    /// Build the request.
    pub fn build(self) -> TextGenerationRequest {
        TextGenerationRequest {
            model: self.model,
            prompt: self.prompt,
            messages: self.messages,
            raw: self.raw,
            stream: self.stream,
            max_tokens: self.max_tokens,
            temperature: self.temperature,
            top_p: self.top_p,
            top_k: self.top_k,
            seed: self.seed,
            repetition_penalty: self.repetition_penalty,
            frequency_penalty: self.frequency_penalty,
            presence_penalty: self.presence_penalty,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Role;

    #[test]
    fn test_builder_basic() {
        let request = TextGenerationRequest::builder("@cf/meta/llama-3-8b-instruct")
            .prompt("Hello")
            .max_tokens(100)
            .temperature(0.7)
            .build();

        assert_eq!(request.model, "@cf/meta/llama-3-8b-instruct");
        assert_eq!(request.prompt, Some("Hello".to_string()));
        assert_eq!(request.max_tokens, Some(100));
        assert_eq!(request.temperature, Some(0.7));
    }

    #[test]
    fn test_builder_messages() {
        let request = TextGenerationRequest::builder("@cf/meta/llama-3-8b-instruct")
            .system("You are helpful")
            .user("Hello")
            .max_tokens(100)
            .build();

        assert!(request.prompt.is_none());
        assert!(request.messages.is_some());
        let messages = request.messages.unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, Role::System);
        assert_eq!(messages[1].role, Role::User);
    }

    #[test]
    fn test_temperature_clamping() {
        let request = TextGenerationRequest::builder("@cf/meta/llama-3-8b-instruct")
            .temperature(10.0) // Above max
            .build();

        assert_eq!(request.temperature, Some(5.0)); // Should be clamped to max
    }

    #[test]
    fn test_penalty_clamping() {
        let request = TextGenerationRequest::builder("@cf/meta/llama-3-8b-instruct")
            .prompt("test")
            .frequency_penalty(5.0) // Above max
            .presence_penalty(-5.0) // Below min
            .build();

        assert_eq!(request.frequency_penalty, Some(2.0));
        assert_eq!(request.presence_penalty, Some(-2.0));
    }
}
