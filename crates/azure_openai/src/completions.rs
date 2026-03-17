//! Legacy completions API for Azure OpenAI.
//!
//! This module provides support for the legacy completions API, which is
//! still available for certain models in Azure OpenAI Service.

use crate::client::AzureOpenAIClient;
use crate::error::Result;
use crate::types::TokenUsage;

/// Client for the legacy completions API.
#[derive(Debug)]
pub struct Completions<'a> {
    client: &'a AzureOpenAIClient,
}

impl<'a> Completions<'a> {
    /// Create a new completions client.
    pub fn new(client: &'a AzureOpenAIClient) -> Self {
        Self { client }
    }

    /// Send a completion request.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use azure_openai::{AzureOpenAIClient, CompletionRequest};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = AzureOpenAIClient::new(
    ///     "my-resource",
    ///     "my-deployment",
    ///     "my-api-key",
    /// )?;
    ///
    /// let request = CompletionRequest::builder()
    ///     .prompt("Once upon a time")
    ///     .max_tokens(100)
    ///     .build();
    ///
    /// let response = client.completions().complete(request).await?;
    /// println!("{}", response.text().unwrap_or_default());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let body = serde_json::to_value(&request)?;
        let deployment = self.client.deployment_name().to_string();

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let body = body.clone();
                let path = format!("/openai/deployments/{}/completions", &deployment);
                Box::pin(async move {
                    let response = client.post(&path, body).await?;
                    let body = client.handle_response(response).await?;
                    let completion_response: CompletionResponse = serde_json::from_value(body)?;
                    Ok(completion_response)
                })
            })
            .await
    }

    /// Send a streaming completion request.
    #[cfg(feature = "streaming")]
    pub async fn stream(
        &self,
        mut request: CompletionRequest,
    ) -> Result<impl futures::Stream<Item = Result<CompletionStreamChunk>>> {
        use eventsource_stream::Eventsource;
        use futures::StreamExt;

        request.stream = Some(true);
        let body = serde_json::to_value(&request)?;
        let deployment = self.client.deployment_name().to_string();
        let url = self.client.build_url(&format!("/openai/deployments/{}/completions", deployment));

        tracing::debug!("Initiating streaming completion request");

        let mut request_builder = reqwest::Client::new()
            .post(&url)
            .header("Content-Type", "application/json");

        // Add authentication headers
        if self.client.is_azure_ad() {
            let auth_header = self.client.config().authorization_header().await?;
            request_builder = request_builder.header("Authorization", auth_header);
        } else {
            if let crate::AzureCredential::ApiKey(key) = &self.client.config().credential {
                request_builder = request_builder.header("api-key", key);
            }
        }

        let response = request_builder
            .json(&body)
            .send()
            .await
            .map_err(crate::error::AzureOpenAIError::from)?;

        let status = response.status();
        if !status.is_success() {
            return Err(crate::error::AzureOpenAIError::from_response(response).await);
        }

        let stream = response
            .bytes_stream()
            .eventsource()
            .map(|event| {
                match event {
                    Ok(event) => {
                        if event.data == "[DONE]" {
                            return Ok(CompletionStreamChunk::done());
                        }

                        match serde_json::from_str::<CompletionStreamChunk>(&event.data) {
                            Ok(chunk) => Ok(chunk),
                            Err(e) => Err(crate::error::AzureOpenAIError::Stream {
                                message: format!("Failed to parse SSE event: {e}"),
                            }),
                        }
                    }
                    Err(e) => Err(crate::error::AzureOpenAIError::Stream {
                        message: format!("SSE error: {e}"),
                    }),
                }
            });

        Ok(stream)
    }
}

/// A completion request.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CompletionRequest {
    /// The prompt(s) to generate completions for.
    pub prompt: CompletionPrompt,

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

    /// Echo back the prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub echo: Option<bool>,

    /// Return log probabilities.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<usize>,

    /// Suffix to append to the completion.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,

    /// Best of n completions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best_of: Option<usize>,
}

/// Completion prompt input.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum CompletionPrompt {
    /// Single string prompt.
    Single(String),
    /// Array of string prompts.
    Multiple(Vec<String>),
}

/// A completion response.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CompletionResponse {
    /// Unique identifier for the completion.
    pub id: String,
    /// The object type (always "text_completion").
    pub object: String,
    /// The Unix timestamp when the completion was created.
    pub created: i64,
    /// The model used for completion.
    pub model: String,
    /// The list of completion choices.
    pub choices: Vec<CompletionChoice>,
    /// Usage statistics.
    pub usage: TokenUsage,
}

impl CompletionResponse {
    /// Get the text of the first choice.
    pub fn text(&self) -> Option<&str> {
        self.choices.first().map(|c| c.text.as_str())
    }

    /// Get all completion texts.
    pub fn texts(&self) -> Vec<&str> {
        self.choices.iter().map(|c| c.text.as_str()).collect()
    }
}

/// A completion choice.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CompletionChoice {
    /// The index of this choice.
    pub index: usize,
    /// The generated text.
    pub text: String,
    /// The reason the completion finished.
    pub finish_reason: Option<crate::types::FinishReason>,
    /// Log probabilities (if requested).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<serde_json::Value>,
}

/// A completion stream chunk.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CompletionStreamChunk {
    /// Unique identifier for the chunk.
    pub id: String,
    /// The object type (always "text_completion").
    pub object: String,
    /// The Unix timestamp.
    pub created: i64,
    /// The model used.
    pub model: String,
    /// The list of choices.
    pub choices: Vec<CompletionStreamChoice>,
}

impl CompletionStreamChunk {
    /// Check if this is the final chunk.
    pub fn is_done(&self) -> bool {
        self.choices.is_empty()
    }

    /// Create a done chunk.
    pub fn done() -> Self {
        Self {
            id: String::new(),
            object: "text_completion".to_string(),
            created: 0,
            model: String::new(),
            choices: vec![],
        }
    }

    /// Get the text delta from the first choice.
    pub fn text(&self) -> Option<&str> {
        self.choices.first().map(|c| c.text.as_str())
    }
}

/// A choice in a streaming completion chunk.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CompletionStreamChoice {
    /// The index of this choice.
    pub index: usize,
    /// The text delta.
    pub text: String,
    /// The reason the completion finished.
    pub finish_reason: Option<crate::types::FinishReason>,
    /// Log probabilities (if requested).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<serde_json::Value>,
}

/// Builder for completion requests.
#[derive(Debug, Clone)]
pub struct CompletionRequestBuilder {
    prompt: CompletionPrompt,
    max_tokens: Option<usize>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    n: Option<usize>,
    stream: Option<bool>,
    stop: Option<Vec<String>>,
    presence_penalty: Option<f32>,
    frequency_penalty: Option<f32>,
    user: Option<String>,
    echo: Option<bool>,
    logprobs: Option<usize>,
    suffix: Option<String>,
    best_of: Option<usize>,
}

impl CompletionRequestBuilder {
    /// Create a new builder with a single prompt.
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: CompletionPrompt::Single(prompt.into()),
            max_tokens: None,
            temperature: None,
            top_p: None,
            n: None,
            stream: None,
            stop: None,
            presence_penalty: None,
            frequency_penalty: None,
            user: None,
            echo: None,
            logprobs: None,
            suffix: None,
            best_of: None,
        }
    }

    /// Create a new builder with multiple prompts.
    pub fn multiple(prompts: Vec<String>) -> Self {
        Self {
            prompt: CompletionPrompt::Multiple(prompts),
            max_tokens: None,
            temperature: None,
            top_p: None,
            n: None,
            stream: None,
            stop: None,
            presence_penalty: None,
            frequency_penalty: None,
            user: None,
            echo: None,
            logprobs: None,
            suffix: None,
            best_of: None,
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

    /// Enable echo.
    pub fn echo(mut self, enabled: bool) -> Self {
        self.echo = Some(enabled);
        self
    }

    /// Set logprobs.
    pub fn logprobs(mut self, logprobs: usize) -> Self {
        self.logprobs = Some(logprobs);
        self
    }

    /// Set suffix.
    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = Some(suffix.into());
        self
    }

    /// Set best_of.
    pub fn best_of(mut self, best_of: usize) -> Self {
        self.best_of = Some(best_of);
        self
    }

    /// Build the request.
    pub fn build(self) -> CompletionRequest {
        CompletionRequest {
            prompt: self.prompt,
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
            echo: self.echo,
            logprobs: self.logprobs,
            suffix: self.suffix,
            best_of: self.best_of,
        }
    }
}

impl CompletionRequest {
    /// Create a new completion request builder.
    pub fn builder(prompt: impl Into<String>) -> CompletionRequestBuilder {
        CompletionRequestBuilder::new(prompt)
    }

    /// Create a new completion request builder with multiple prompts.
    pub fn builder_multiple(prompts: Vec<String>) -> CompletionRequestBuilder {
        CompletionRequestBuilder::multiple(prompts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_completion_request_builder() {
        let request = CompletionRequest::builder("Once upon a time")
            .max_tokens(100)
            .temperature(0.7)
            .build();

        match request.prompt {
            CompletionPrompt::Single(s) => assert_eq!(s, "Once upon a time"),
            _ => panic!("Expected single prompt"),
        }
        assert_eq!(request.max_tokens, Some(100));
        assert_eq!(request.temperature, Some(0.7));
    }

    #[test]
    fn test_completion_response_helpers() {
        let response = CompletionResponse {
            id: "cmpl-123".to_string(),
            object: "text_completion".to_string(),
            created: 1234567890,
            model: "davinci".to_string(),
            choices: vec![CompletionChoice {
                index: 0,
                text: "The end.".to_string(),
                finish_reason: Some(crate::types::FinishReason::Stop),
                logprobs: None,
            }],
            usage: TokenUsage::new(10, 5),
        };

        assert_eq!(response.text(), Some("The end."));
    }
}
