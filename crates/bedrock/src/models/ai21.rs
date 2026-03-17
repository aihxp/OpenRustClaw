//! AI21 Labs Jurassic/Jamba model-specific types.

use serde::{Deserialize, Serialize};

/// AI21 Jurassic request body.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ai21JurassicRequest {
    /// The prompt.
    pub prompt: String,
    /// Maximum tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i32>,
    /// Temperature.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Top-p.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Top-k.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k_return: Option<i32>,
    /// Stop sequences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    /// Penalty for repetition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count_penalty: Option<Ai21Penalty>,
    /// Presence penalty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<Ai21Penalty>,
    /// Frequency penalty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<Ai21Penalty>,
    /// Number of completions to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_results: Option<i32>,
}

/// AI21 penalty configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ai21Penalty {
    /// Scale of the penalty (0.0 - 1.0).
    pub scale: f32,
    /// Whether to apply to numbers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apply_to_numbers: Option<bool>,
    /// Whether to apply to punctuation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apply_to_punctuation: Option<bool>,
    /// Whether to apply to stop words.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apply_to_stop_words: Option<bool>,
    /// Whether to apply to whitespaces.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apply_to_whitespaces: Option<bool>,
    /// Whether to apply to emojis.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apply_to_emojis: Option<bool>,
}

/// AI21 Jurassic response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ai21JurassicResponse {
    /// The completions.
    pub completions: Vec<Ai21Completion>,
    /// The ID.
    pub id: i64,
}

/// AI21 completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ai21Completion {
    /// The generated data.
    pub data: Ai21CompletionData,
    /// Finish reason.
    pub finish_reason: Option<Ai21FinishReason>,
}

/// AI21 completion data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ai21CompletionData {
    /// The generated text.
    pub text: String,
    /// Generated tokens.
    pub tokens: Vec<Ai21Token>,
}

/// AI21 token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ai21Token {
    /// The generated token.
    pub generated_token: Ai21GeneratedToken,
}

/// AI21 generated token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ai21GeneratedToken {
    /// The token.
    pub token: String,
    /// The log probability.
    pub logprob: Option<f32>,
    /// Raw token.
    pub raw_logprob: Option<f32>,
}

/// AI21 finish reason.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ai21FinishReason {
    /// The reason.
    pub reason: String,
}

impl Ai21JurassicRequest {
    /// Create a new Jurassic request.
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            max_tokens: None,
            temperature: None,
            top_p: None,
            top_k_return: None,
            stop_sequences: None,
            count_penalty: None,
            presence_penalty: None,
            frequency_penalty: None,
            num_results: None,
        }
    }

    /// Set temperature.
    pub fn temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature.clamp(0.0, 1.0));
        self
    }

    /// Set max tokens.
    pub fn max_tokens(mut self, max_tokens: i32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }
}

impl Ai21JurassicResponse {
    /// Get the first completion text.
    pub fn text(&self) -> String {
        self.completions
            .first()
            .map(|c| c.data.text.clone())
            .unwrap_or_default()
    }
}

/// AI21 Jamba request body.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ai21JambaRequest {
    /// The messages.
    pub messages: Vec<Ai21Message>,
    /// Maximum tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i32>,
    /// Temperature.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Top-p.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Stop sequences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    /// Number of completions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<i32>,
}

/// AI21 message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ai21Message {
    /// The role.
    pub role: Ai21Role,
    /// The content.
    pub content: String,
}

/// AI21 role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Ai21Role {
    /// System message.
    System,
    /// User message.
    User,
    /// Assistant message.
    Assistant,
}

/// AI21 Jamba response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ai21JambaResponse {
    /// The choices.
    pub choices: Vec<Ai21Choice>,
    /// Usage statistics.
    pub usage: Ai21Usage,
}

/// AI21 choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ai21Choice {
    /// The index.
    pub index: i32,
    /// The message.
    pub message: Ai21Message,
    /// Finish reason.
    pub finish_reason: Option<String>,
}

/// AI21 usage.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Ai21Usage {
    /// Prompt tokens.
    pub prompt_tokens: i32,
    /// Completion tokens.
    pub completion_tokens: i32,
    /// Total tokens.
    pub total_tokens: i32,
}

impl Ai21JambaRequest {
    /// Create a new Jamba request.
    pub fn new(messages: Vec<Ai21Message>) -> Self {
        Self {
            messages,
            max_tokens: None,
            temperature: None,
            top_p: None,
            stop: None,
            n: None,
        }
    }

    /// Set temperature.
    pub fn temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature.clamp(0.0, 1.0));
        self
    }

    /// Set max tokens.
    pub fn max_tokens(mut self, max_tokens: i32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }
}

impl Ai21JambaResponse {
    /// Get the first choice text.
    pub fn text(&self) -> String {
        self.choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default()
    }
}
