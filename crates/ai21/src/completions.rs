//! Completions API for AI21's Jurassic models.

use serde::{Deserialize, Serialize};

use crate::client::Ai21Client;
use crate::constants::endpoints;
use crate::error::Result;
use crate::types::Penalty;

/// Client for the Completions API.
#[derive(Debug)]
pub struct CompletionsEndpoint<'a> {
    pub(crate) client: &'a Ai21Client,
}

impl<'a> CompletionsEndpoint<'a> {
    /// Send a completion request and get a response.
    pub async fn create(&self, request: CompletionRequest) -> Result<CompletionResponse> {
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
}

/// A request to the AI21 Completions API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    /// The model to use (e.g., "j2-ultra").
    pub model: String,

    /// The prompt to complete.
    pub prompt: String,

    /// The maximum number of tokens to generate.
    #[serde(rename = "maxTokens", skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<usize>,

    /// Temperature for sampling (0.0 to 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Top-p sampling parameter (0.0 to 1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,

    /// Nucleus sampling: limit to top K tokens.
    #[serde(rename = "topKReturn", skip_serializing_if = "Option::is_none")]
    pub top_k_return: Option<usize>,

    /// Stop sequences.
    #[serde(rename = "stopSequences", skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,

    /// Random seed for reproducibility.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,

    /// Number of completions to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_results: Option<usize>,

    /// Presence penalty configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<Penalty>,

    /// Count penalty configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count_penalty: Option<Penalty>,

    /// Frequency penalty configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<Penalty>,

    /// Minimum time per token in milliseconds.
    #[serde(rename = "minTokens", skip_serializing_if = "Option::is_none")]
    pub min_tokens: Option<usize>,

    /// Whether to include the prompt in the response.
    #[serde(rename = "echo", skip_serializing_if = "Option::is_none")]
    pub echo: Option<bool>,
}

impl CompletionRequest {
    /// Create a new request builder for the given model.
    pub fn builder(model: impl Into<String>) -> CompletionRequestBuilder {
        CompletionRequestBuilder::new(model)
    }

    /// Create a simple request with just a prompt.
    pub fn simple(model: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self::builder(model).prompt(prompt).build()
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
    top_k_return: Option<usize>,
    stop_sequences: Option<Vec<String>>,
    seed: Option<u64>,
    num_results: Option<usize>,
    presence_penalty: Option<Penalty>,
    count_penalty: Option<Penalty>,
    frequency_penalty: Option<Penalty>,
    min_tokens: Option<usize>,
    echo: Option<bool>,
}

impl CompletionRequestBuilder {
    /// Create a new builder for the given model.
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            prompt: String::new(),
            max_tokens: None,
            temperature: None,
            top_p: None,
            top_k_return: None,
            stop_sequences: None,
            seed: None,
            num_results: None,
            presence_penalty: None,
            count_penalty: None,
            frequency_penalty: None,
            min_tokens: None,
            echo: None,
        }
    }

    /// Set the prompt.
    pub fn prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = prompt.into();
        self
    }

    /// Set the maximum tokens.
    pub fn max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set the temperature.
    pub fn temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp.clamp(0.0, 2.0));
        self
    }

    /// Set the top-p parameter.
    pub fn top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p.clamp(0.0, 1.0));
        self
    }

    /// Set the top-k return parameter.
    pub fn top_k_return(mut self, top_k: usize) -> Self {
        self.top_k_return = Some(top_k);
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

    /// Set the random seed.
    pub fn seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Set the number of results.
    pub fn num_results(mut self, num: usize) -> Self {
        self.num_results = Some(num);
        self
    }

    /// Set the presence penalty.
    pub fn presence_penalty(mut self, penalty: Penalty) -> Self {
        self.presence_penalty = Some(penalty);
        self
    }

    /// Set the count penalty.
    pub fn count_penalty(mut self, penalty: Penalty) -> Self {
        self.count_penalty = Some(penalty);
        self
    }

    /// Set the frequency penalty.
    pub fn frequency_penalty(mut self, penalty: Penalty) -> Self {
        self.frequency_penalty = Some(penalty);
        self
    }

    /// Set the minimum tokens.
    pub fn min_tokens(mut self, min_tokens: usize) -> Self {
        self.min_tokens = Some(min_tokens);
        self
    }

    /// Set whether to echo the prompt.
    pub fn echo(mut self, echo: bool) -> Self {
        self.echo = Some(echo);
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
            top_k_return: self.top_k_return,
            stop_sequences: self.stop_sequences,
            seed: self.seed,
            num_results: self.num_results,
            presence_penalty: self.presence_penalty,
            count_penalty: self.count_penalty,
            frequency_penalty: self.frequency_penalty,
            min_tokens: self.min_tokens,
            echo: self.echo,
        }
    }
}

/// A response from the AI21 Completions API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    /// The ID of the response.
    pub id: String,

    /// The prompt information.
    pub prompt: PromptInfo,

    /// The list of completions.
    pub completions: Vec<Completion>,
}

impl CompletionResponse {
    /// Get the text of the first completion.
    pub fn text(&self) -> Option<&str> {
        self.completions.first().map(|c| c.data.text.as_str())
    }

    /// Get all completion texts.
    pub fn all_texts(&self) -> Vec<&str> {
        self.completions
            .iter()
            .map(|c| c.data.text.as_str())
            .collect()
    }
}

/// Information about the prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptInfo {
    /// The prompt text.
    pub text: String,

    /// Token information for the prompt.
    pub tokens: Vec<TokenInfo>,
}

/// A single completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Completion {
    /// The completion data.
    pub data: CompletionData,

    /// Token information for the completion.
    pub tokens: Vec<TokenInfo>,
}

/// The data for a completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionData {
    /// The generated text.
    pub text: String,

    /// The tokens generated.
    pub tokens: Vec<TokenData>,
}

/// Token information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    /// The token ID.
    #[serde(rename = "token")]
    pub token_id: String,

    /// The token text.
    #[serde(rename = "text")]
    pub token_text: String,

    /// The log probability of the token.
    #[serde(rename = "logprob")]
    pub log_prob: Option<f64>,
}

/// Token data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenData {
    /// The token text.
    #[serde(rename = "generatedToken")]
    pub generated_token: GeneratedToken,

    /// The top alternative tokens.
    #[serde(rename = "topTokens")]
    pub top_tokens: Option<Vec<TopToken>>,

    /// The reason for the token.
    #[serde(rename = "reason")]
    pub reason: String,
}

/// A generated token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedToken {
    /// The token text.
    pub token: String,

    /// The token ID.
    #[serde(rename = "logprob")]
    pub log_prob: Option<f64>,

    /// The raw token representation.
    #[serde(rename = "rawRepresentation")]
    pub raw_representation: Option<String>,
}

/// A top alternative token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopToken {
    /// The token text.
    pub token: String,

    /// The log probability.
    #[serde(rename = "logprob")]
    pub log_prob: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_basic() {
        let request = CompletionRequest::builder("j2-ultra")
            .prompt("Hello")
            .temperature(0.7)
            .max_tokens(100)
            .build();

        assert_eq!(request.model, "j2-ultra");
        assert_eq!(request.prompt, "Hello");
        assert_eq!(request.temperature, Some(0.7));
        assert_eq!(request.max_tokens, Some(100));
    }

    #[test]
    fn test_builder_with_penalties() {
        let request = CompletionRequest::builder("j2-mid")
            .prompt("Test")
            .presence_penalty(Penalty::new(0.5).with_numbers(true))
            .frequency_penalty(Penalty::new(0.3))
            .build();

        assert!(request.presence_penalty.is_some());
        assert!(request.frequency_penalty.is_some());
        assert_eq!(request.presence_penalty.as_ref().unwrap().scale, Some(0.5));
    }

    #[test]
    fn test_response_helpers() {
        let response = CompletionResponse {
            id: "comp_123".to_string(),
            prompt: PromptInfo {
                text: "Hello".to_string(),
                tokens: vec![],
            },
            completions: vec![Completion {
                data: CompletionData {
                    text: " World!".to_string(),
                    tokens: vec![],
                },
                tokens: vec![],
            }],
        };

        assert_eq!(response.text(), Some(" World!"));
        assert_eq!(response.all_texts(), vec![" World!"]);
    }
}
