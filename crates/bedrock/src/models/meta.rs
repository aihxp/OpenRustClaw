//! Meta Llama model-specific types.

use serde::{Deserialize, Serialize};

/// Meta Llama request body for InvokeModel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlamaRequest {
    /// The prompt text.
    pub prompt: String,
    /// Temperature (0.0 - 1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Top-p sampling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Maximum number of tokens to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_gen_len: Option<i32>,
}

/// Meta Llama response body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlamaResponse {
    /// The generated text.
    pub generation: String,
    /// The prompt token count.
    pub prompt_token_count: i32,
    /// The generation token count.
    pub generation_token_count: i32,
    /// The stop reason.
    pub stop_reason: String,
}

impl LlamaRequest {
    /// Create a new Llama request.
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            temperature: None,
            top_p: None,
            max_gen_len: None,
        }
    }

    /// Set temperature.
    pub fn temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature.clamp(0.0, 1.0));
        self
    }

    /// Set top-p.
    pub fn top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p.clamp(0.0, 1.0));
        self
    }

    /// Set max tokens.
    pub fn max_tokens(mut self, max_tokens: i32) -> Self {
        self.max_gen_len = Some(max_tokens);
        self
    }

    /// Format a chat prompt from messages.
    pub fn from_messages(messages: &[LlamaMessage]) -> String {
        let mut prompt = String::new();
        for msg in messages {
            prompt.push_str(&msg.format());
        }
        prompt
    }
}

/// A message in Llama chat format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlamaMessage {
    /// The role.
    pub role: LlamaRole,
    /// The content.
    pub content: String,
}

/// Llama role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LlamaRole {
    /// System message.
    System,
    /// User message.
    User,
    /// Assistant message.
    Assistant,
}

impl LlamaMessage {
    /// Create a new message.
    pub fn new(role: LlamaRole, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
        }
    }

    /// Format the message for the prompt.
    pub fn format(&self) -> String {
        match self.role {
            LlamaRole::System => format!("<|system|>\n{}\n", self.content),
            LlamaRole::User => format!("<|user|>\n{}\n", self.content),
            LlamaRole::Assistant => format!("<|assistant|>\n{}\n", self.content),
        }
    }
}
