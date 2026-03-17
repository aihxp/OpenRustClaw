//! Generate completions API for Ollama.

use crate::client::OllamaClient;
use crate::client::endpoints;
use crate::error::{OllamaError, Result};
use crate::types::{FormatType, GenerateResponse, ImageInput, KeepAlive, Options};

/// Client for the generate API.
#[derive(Debug)]
pub struct Generate<'a> {
    client: &'a OllamaClient,
}

impl<'a> Generate<'a> {
    /// Create a new generate client.
    pub fn new(client: &'a OllamaClient) -> Self {
        Self { client }
    }

    /// Send a generate completion request.
    ///
    /// This is the legacy completion endpoint that takes a single prompt
    /// rather than a conversation of messages.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ollama_sdk::{OllamaClient, GenerateRequest};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = OllamaClient::new("http://localhost:11434");
    ///
    /// let request = GenerateRequest::builder("llama3.2")
    ///     .prompt("Why is the sky blue?")
    ///     .build();
    ///
    /// let response = client.generate().text(request).await?;
    /// println!("{}", response.response);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn text(&self, request: GenerateRequest) -> Result<GenerateResponse> {
        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let body = request.clone();
                Box::pin(async move {
                    let response = client.post(endpoints::GENERATE, body).await?;
                    let gen_response: GenerateResponse = client.handle_response(response).await?;
                    Ok(gen_response)
                })
            })
            .await
    }

    /// Send a streaming generate completion request.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ollama_sdk::{OllamaClient, GenerateRequest};
    /// use futures::StreamExt;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = OllamaClient::new("http://localhost:11434");
    ///
    /// let request = GenerateRequest::builder("llama3.2")
    ///     .prompt("Tell me a story")
    ///     .build();
    ///
    /// let mut stream = client.generate().stream(request).await?;
    /// while let Some(chunk) = stream.next().await {
    ///     match chunk {
    ///         Ok(chunk) => print!("{}", chunk.response),
    ///         Err(e) => eprintln!("Error: {}", e),
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "streaming")]
    pub async fn stream(&self, mut request: GenerateRequest) -> Result<GenerateStream> {
        use futures::StreamExt;

        request.stream = Some(true);

        let url = format!("{}{}", self.client.base_url(), endpoints::GENERATE);

        tracing::debug!("Initiating streaming generate request");

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
            .map(|bytes| match bytes {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    let lines: Vec<&str> = text.lines().collect();
                    let results: Vec<Result<GenerateStreamChunk>> = lines
                        .into_iter()
                        .filter(|line| !line.is_empty())
                        .map(
                            |line| match serde_json::from_str::<GenerateStreamChunk>(line) {
                                Ok(chunk) => Ok(chunk),
                                Err(e) => Err(OllamaError::Stream {
                                    message: format!("Failed to parse NDJSON: {e}"),
                                }),
                            },
                        )
                        .collect();

                    futures::stream::iter(results)
                }
                Err(e) => futures::stream::iter(vec![Err(OllamaError::Stream {
                    message: format!("Stream error: {e}"),
                })]),
            })
            .flatten();

        Ok(GenerateStream::new(stream))
    }
}

/// A generate completion request.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GenerateRequest {
    /// The model name.
    pub model: String,
    /// The prompt to generate from.
    pub prompt: String,
    /// Images for multi-modal models (base64 encoded).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<String>>,
    /// Format for the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<FormatType>,
    /// System prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// Template to use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    /// Context from previous requests (for maintaining state).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Vec<u64>>,
    /// Whether to stream the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    /// Raw mode (for raw model access).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<bool>,
    /// Additional model parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Options>,
    /// Controls how long the model stays loaded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_alive: Option<KeepAlive>,
}

impl GenerateRequest {
    /// Create a new generate request builder.
    pub fn builder(model: impl Into<String>) -> GenerateRequestBuilder {
        GenerateRequestBuilder::new(model)
    }
}

/// Builder for generate requests.
#[derive(Debug, Clone)]
pub struct GenerateRequestBuilder {
    model: String,
    prompt: String,
    images: Option<Vec<String>>,
    format: Option<FormatType>,
    system: Option<String>,
    template: Option<String>,
    context: Option<Vec<u64>>,
    stream: Option<bool>,
    raw: Option<bool>,
    options: Option<Options>,
    keep_alive: Option<KeepAlive>,
}

impl GenerateRequestBuilder {
    /// Create a new builder.
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            prompt: String::new(),
            images: None,
            format: None,
            system: None,
            template: None,
            context: None,
            stream: None,
            raw: None,
            options: None,
            keep_alive: None,
        }
    }

    /// Set the prompt.
    pub fn prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = prompt.into();
        self
    }

    /// Set images for vision models.
    pub fn images(mut self, images: Vec<ImageInput>) -> Self {
        self.images = Some(
            images
                .into_iter()
                .filter_map(|img| img.to_base64().ok())
                .collect(),
        );
        self
    }

    /// Add a single image.
    pub fn image(mut self, image: ImageInput) -> Self {
        if let Ok(encoded) = image.to_base64() {
            self.images.get_or_insert_with(Vec::new).push(encoded);
        }
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

    /// Set the system prompt.
    pub fn system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(system.into());
        self
    }

    /// Set the template.
    pub fn template(mut self, template: impl Into<String>) -> Self {
        self.template = Some(template.into());
        self
    }

    /// Set the context.
    pub fn context(mut self, context: Vec<u64>) -> Self {
        self.context = Some(context);
        self
    }

    /// Enable raw mode.
    pub fn raw(mut self, enabled: bool) -> Self {
        self.raw = Some(enabled);
        self
    }

    /// Enable streaming.
    pub fn stream(mut self, enabled: bool) -> Self {
        self.stream = Some(enabled);
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

    /// Set the keep_alive duration.
    pub fn keep_alive(mut self, keep_alive: KeepAlive) -> Self {
        self.keep_alive = Some(keep_alive);
        self
    }

    /// Build the request.
    pub fn build(self) -> GenerateRequest {
        GenerateRequest {
            model: self.model,
            prompt: self.prompt,
            images: self.images,
            format: self.format,
            system: self.system,
            template: self.template,
            context: self.context,
            stream: self.stream,
            raw: self.raw,
            options: self.options,
            keep_alive: self.keep_alive,
        }
    }
}

/// A generate completion chunk in a stream.
#[cfg(feature = "streaming")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GenerateStreamChunk {
    /// The model used.
    pub model: String,
    /// The creation timestamp.
    pub created_at: String,
    /// The response text.
    pub response: String,
    /// Whether this is the final chunk.
    pub done: bool,
    /// Context for subsequent requests.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Vec<u64>>,
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
impl GenerateStreamChunk {
    /// Check if this is the final chunk.
    pub fn is_done(&self) -> bool {
        self.done
    }
}

/// A streaming generate completion response.
#[cfg(feature = "streaming")]
pub struct GenerateStream {
    inner: std::pin::Pin<Box<dyn futures::Stream<Item = Result<GenerateStreamChunk>> + Send>>,
}

#[cfg(feature = "streaming")]
impl GenerateStream {
    fn new<S>(stream: S) -> Self
    where
        S: futures::Stream<Item = Result<GenerateStreamChunk>> + Send + 'static,
    {
        Self {
            inner: Box::pin(stream),
        }
    }
}

#[cfg(feature = "streaming")]
impl futures::Stream for GenerateStream {
    type Item = Result<GenerateStreamChunk>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}

#[cfg(feature = "streaming")]
impl std::fmt::Debug for GenerateStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GenerateStream").finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_basic() {
        let request = GenerateRequest::builder("llama3.2").prompt("Hello").build();

        assert_eq!(request.model, "llama3.2");
        assert_eq!(request.prompt, "Hello");
    }

    #[test]
    fn test_builder_with_options() {
        let request = GenerateRequest::builder("llama3.2")
            .prompt("Hello")
            .temperature(0.7)
            .max_tokens(1024)
            .json_mode()
            .system("You are helpful")
            .build();

        assert!(request.format.is_some());
        assert_eq!(request.system, Some("You are helpful".to_string()));
        assert!(request.options.is_some());
    }

    #[test]
    fn test_builder_raw_mode() {
        let request = GenerateRequest::builder("llama3.2")
            .prompt("Hello")
            .raw(true)
            .build();

        assert_eq!(request.raw, Some(true));
    }

    #[test]
    fn test_keep_alive() {
        let request = GenerateRequest::builder("llama3.2")
            .prompt("Hello")
            .keep_alive(KeepAlive::duration("5m"))
            .build();

        assert!(matches!(request.keep_alive, Some(KeepAlive::String(s)) if s == "5m"));
    }
}
