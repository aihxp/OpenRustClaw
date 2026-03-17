//! Legacy completions API for Fireworks AI.

use crate::client::FireworksClient;
use crate::constants::endpoints;
use crate::error::Result;
use crate::types::{CompletionResponse, ResponseFormat};

/// Client for completions.
#[derive(Debug)]
pub struct Completions<'a> {
    client: &'a FireworksClient,
}

impl<'a> Completions<'a> {
    /// Create a new completions client.
    pub fn new(client: &'a FireworksClient) -> Self {
        Self { client }
    }

    /// Send a completion request.
    pub async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let body = serde_json::to_value(&request)?;

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let body = body.clone();
                Box::pin(async move {
                    let response = client.post(endpoints::COMPLETIONS, body).await?;
                    let body = client.handle_response(response).await?;
                    let completion_response: CompletionResponse = serde_json::from_value(body)?;
                    Ok(completion_response)
                })
            })
            .await
    }

    /// Send a streaming completion request.
    #[cfg(feature = "streaming")]
    pub async fn complete_stream(
        &self,
        request: CompletionRequest,
    ) -> Result<crate::streaming::CompletionStream> {
        use crate::streaming::CompletionStream;

        let mut request = request;
        request.stream = Some(true);

        let body = serde_json::to_value(&request)?;

        let response = self.client.post(endpoints::COMPLETIONS, body).await?;

        if !response.status().is_success() {
            return Err(crate::error::FireworksError::from_response(response).await);
        }

        Ok(CompletionStream::new(response))
    }
}

/// A completion request.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CompletionRequest {
    /// Model ID.
    pub model: String,
    /// Prompt text.
    pub prompt: String,
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
    /// Random seed for reproducibility.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
    /// Suffix to append to the generated text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,
}

impl CompletionRequest {
    /// Create a new builder.
    pub fn builder(
        model: impl Into<String>,
        prompt: impl Into<String>,
    ) -> CompletionRequestBuilder {
        CompletionRequestBuilder::new(model, prompt)
    }

    /// Create a simple request.
    pub fn simple(model: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self::builder(model, prompt).build()
    }
}

/// Builder for completion requests.
#[derive(Debug, Clone)]
pub struct CompletionRequestBuilder {
    model: String,
    prompt: String,
    max_tokens: Option<usize>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    top_k: Option<usize>,
    stream: Option<bool>,
    stop: Option<Vec<String>>,
    presence_penalty: Option<f32>,
    frequency_penalty: Option<f32>,
    logit_bias: Option<crate::types::LogitBias>,
    logprobs: Option<bool>,
    top_logprobs: Option<usize>,
    n: Option<usize>,
    response_format: Option<ResponseFormat>,
    seed: Option<i64>,
    suffix: Option<String>,
}

impl CompletionRequestBuilder {
    /// Create a new builder.
    pub fn new(model: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            prompt: prompt.into(),
            max_tokens: None,
            temperature: None,
            top_p: None,
            top_k: None,
            stream: None,
            stop: None,
            presence_penalty: None,
            frequency_penalty: None,
            logit_bias: None,
            logprobs: None,
            top_logprobs: None,
            n: None,
            response_format: None,
            seed: None,
            suffix: None,
        }
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

    /// Set random seed.
    pub fn seed(mut self, seed: i64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Set suffix.
    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = Some(suffix.into());
        self
    }

    /// Build the request.
    pub fn build(self) -> CompletionRequest {
        CompletionRequest {
            model: self.model,
            prompt: self.prompt,
            max_tokens: self.max_tokens,
            temperature: self.temperature,
            top_p: self.top_p,
            top_k: self.top_k,
            stream: self.stream,
            stop: self.stop,
            presence_penalty: self.presence_penalty,
            frequency_penalty: self.frequency_penalty,
            logit_bias: self.logit_bias,
            logprobs: self.logprobs,
            top_logprobs: self.top_logprobs,
            n: self.n,
            response_format: self.response_format,
            seed: self.seed,
            suffix: self.suffix,
        }
    }
}
