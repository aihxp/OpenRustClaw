//! Generate API for Cohere's Command models (legacy).

use serde::{Deserialize, Serialize};

use crate::client::CohereClient;
use crate::constants::endpoints;
use crate::error::Result;
use crate::types::{ApiMeta, FinishReason};

/// Client for the Generate API.
#[derive(Debug)]
pub struct GenerateEndpoint<'a> {
    pub(crate) client: &'a CohereClient,
}

impl<'a> GenerateEndpoint<'a> {
    /// Send a generate request and get a complete response.
    pub async fn create(&self, request: GenerateRequest) -> Result<GenerateResponse> {
        let body = serde_json::to_value(&request)?;

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let body = body.clone();
                Box::pin(async move {
                    let response = client.post(endpoints::GENERATE, body).await?;
                    let body = client.handle_response(response).await?;
                    let generate_response: GenerateResponse = serde_json::from_value(body)?;
                    Ok(generate_response)
                })
            })
            .await
    }
}

/// A request to the Cohere Generate API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateRequest {
    /// The model to use (e.g., "command").
    pub model: String,

    /// The prompt to generate from.
    pub prompt: String,

    /// Whether to stream the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    /// Temperature for sampling (0.0 to 5.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Number of generations to return.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_generations: Option<usize>,

    /// Enable log probabilities.
    #[serde(rename = "return_likelihoods", skip_serializing_if = "Option::is_none")]
    pub return_likelihoods: Option<String>,

    /// Top-k sampling parameter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub k: Option<usize>,

    /// Top-p sampling parameter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub p: Option<f32>,

    /// Frequency penalty (0.0 to 1.0).
    #[serde(rename = "frequency_penalty", skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,

    /// Presence penalty (0.0 to 1.0).
    #[serde(rename = "presence_penalty", skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,

    /// Stop sequences.
    #[serde(rename = "stop_sequences", skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,

    /// Maximum number of tokens to generate.
    #[serde(rename = "max_tokens", skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<usize>,

    /// Truncate the prompt if it's too long.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncate: Option<String>,

    /// Random seed for reproducibility.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,

    /// End sequences to stop at.
    #[serde(rename = "end_sequences", skip_serializing_if = "Option::is_none")]
    pub end_sequences: Option<Vec<String>>,
}

impl GenerateRequest {
    /// Create a new request builder for the given model.
    pub fn builder(model: impl Into<String>) -> GenerateRequestBuilder {
        GenerateRequestBuilder::new(model)
    }

    /// Create a simple request with a single prompt.
    pub fn simple(model: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self::builder(model).prompt(prompt).build()
    }
}

/// Builder for generate requests.
#[derive(Debug, Clone)]
pub struct GenerateRequestBuilder {
    model: String,
    prompt: String,
    stream: Option<bool>,
    temperature: Option<f32>,
    num_generations: Option<usize>,
    return_likelihoods: Option<String>,
    k: Option<usize>,
    p: Option<f32>,
    frequency_penalty: Option<f32>,
    presence_penalty: Option<f32>,
    stop_sequences: Option<Vec<String>>,
    max_tokens: Option<usize>,
    truncate: Option<String>,
    seed: Option<u64>,
    end_sequences: Option<Vec<String>>,
}

impl GenerateRequestBuilder {
    /// Create a new builder for the given model.
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            prompt: String::new(),
            stream: None,
            temperature: None,
            num_generations: None,
            return_likelihoods: None,
            k: None,
            p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop_sequences: None,
            max_tokens: None,
            truncate: None,
            seed: None,
            end_sequences: None,
        }
    }

    /// Set the prompt.
    pub fn prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = prompt.into();
        self
    }

    /// Enable/disable streaming.
    pub fn stream(mut self, enabled: bool) -> Self {
        self.stream = Some(enabled);
        self
    }

    /// Set the temperature.
    pub fn temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp.clamp(0.0, 5.0));
        self
    }

    /// Set the number of generations.
    pub fn num_generations(mut self, n: usize) -> Self {
        self.num_generations = Some(n);
        self
    }

    /// Enable returning likelihoods.
    pub fn return_likelihoods(mut self, mode: impl Into<String>) -> Self {
        self.return_likelihoods = Some(mode.into());
        self
    }

    /// Set the top-k parameter.
    pub fn k(mut self, k: usize) -> Self {
        self.k = Some(k);
        self
    }

    /// Set the top-p parameter.
    pub fn p(mut self, p: f32) -> Self {
        self.p = Some(p.clamp(0.0, 1.0));
        self
    }

    /// Set the frequency penalty.
    pub fn frequency_penalty(mut self, penalty: f32) -> Self {
        self.frequency_penalty = Some(penalty.clamp(0.0, 1.0));
        self
    }

    /// Set the presence penalty.
    pub fn presence_penalty(mut self, penalty: f32) -> Self {
        self.presence_penalty = Some(penalty.clamp(0.0, 1.0));
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

    /// Set the maximum tokens.
    pub fn max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set the truncation mode.
    pub fn truncate(mut self, mode: impl Into<String>) -> Self {
        self.truncate = Some(mode.into());
        self
    }

    /// Set the random seed.
    pub fn seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Add an end sequence.
    pub fn end_sequence(mut self, seq: impl Into<String>) -> Self {
        self.end_sequences
            .get_or_insert_with(Vec::new)
            .push(seq.into());
        self
    }

    /// Set the end sequences.
    pub fn end_sequences(mut self, seqs: Vec<String>) -> Self {
        self.end_sequences = Some(seqs);
        self
    }

    /// Build the request.
    pub fn build(self) -> GenerateRequest {
        GenerateRequest {
            model: self.model,
            prompt: self.prompt,
            stream: self.stream,
            temperature: self.temperature,
            num_generations: self.num_generations,
            return_likelihoods: self.return_likelihoods,
            k: self.k,
            p: self.p,
            frequency_penalty: self.frequency_penalty,
            presence_penalty: self.presence_penalty,
            stop_sequences: self.stop_sequences,
            max_tokens: self.max_tokens,
            truncate: self.truncate,
            seed: self.seed,
            end_sequences: self.end_sequences,
        }
    }
}

/// A response from the Cohere Generate API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateResponse {
    /// The generated text (if num_generations=1).
    pub text: Option<String>,

    /// Multiple generations (if num_generations>1).
    pub generations: Option<Vec<Generation>>,

    /// Generation ID.
    #[serde(rename = "id")]
    pub generation_id: String,

    /// Prompt tokens.
    #[serde(rename = "prompt", skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,

    /// API metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<ApiMeta>,
}

impl GenerateResponse {
    /// Get the generated text (works for single or multiple generations).
    pub fn text(&self) -> Option<&str> {
        self.text
            .as_deref()
            .or_else(|| self.generations.as_ref().and_then(|g| g.first().map(|g| g.text.as_str())))
    }

    /// Get all generations.
    pub fn get_generations(&self) -> &[Generation] {
        self.generations.as_deref().unwrap_or(&[])
    }
}

/// A single generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Generation {
    /// The generated text.
    pub text: String,

    /// The finish reason.
    #[serde(rename = "finish_reason", skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<FinishReason>,

    /// Token likelihoods.
    #[serde(rename = "token_likelihoods", skip_serializing_if = "Option::is_none")]
    pub token_likelihoods: Option<Vec<TokenLikelihood>>,
}

/// Token likelihood.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenLikelihood {
    /// The token.
    pub token: String,
    /// The likelihood of the token.
    pub likelihood: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_basic() {
        let request = GenerateRequest::builder("command")
            .prompt("Once upon a time")
            .temperature(0.7)
            .max_tokens(100)
            .build();

        assert_eq!(request.model, "command");
        assert_eq!(request.prompt, "Once upon a time");
        assert_eq!(request.temperature, Some(0.7));
        assert_eq!(request.max_tokens, Some(100));
    }

    #[test]
    fn test_builder_chain() {
        let request = GenerateRequest::builder("command-nightly")
            .prompt("Hello")
            .num_generations(3)
            .k(50)
            .p(0.9)
            .stop_sequence("END")
            .seed(42)
            .build();

        assert_eq!(request.num_generations, Some(3));
        assert_eq!(request.k, Some(50));
        assert_eq!(request.p, Some(0.9));
        assert_eq!(request.seed, Some(42));
    }

    #[test]
    fn test_response_helpers() {
        let response = GenerateResponse {
            text: Some("Hello world".to_string()),
            generations: None,
            generation_id: "gen_123".to_string(),
            prompt: None,
            meta: None,
        };

        assert_eq!(response.text(), Some("Hello world"));
    }
}
