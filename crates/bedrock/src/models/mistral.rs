//! Mistral AI model-specific types.

use serde::{Deserialize, Serialize};

/// Mistral request body for InvokeModel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MistralRequest {
    /// The prompt.
    pub prompt: String,
    /// Maximum tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i32>,
    /// Stop sequences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    /// Temperature.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Top-p.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Top-k.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i32>,
}

/// Mistral response body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MistralResponse {
    /// The generated outputs.
    pub outputs: Vec<MistralOutput>,
}

/// Mistral output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MistralOutput {
    /// The generated text.
    pub text: String,
    /// Stop reason.
    pub stop_reason: String,
}

impl MistralRequest {
    /// Create a new Mistral request.
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            max_tokens: None,
            stop: None,
            temperature: None,
            top_p: None,
            top_k: None,
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

    /// Format messages into Mistral prompt format.
    ///
    /// Mistral format: `<s>[INST] {system_prompt}\n{user_message} [/INST] {assistant_response}</s>[INST] {user_message} [/INST]`
    pub fn format_prompt(system: Option<&str>, messages: &[MistralMessage]) -> String {
        let mut prompt = String::from("<s>");

        let mut i = 0;
        while i < messages.len() {
            let msg = &messages[i];

            match msg.role {
                MistralRole::User => {
                    prompt.push_str("[INST] ");
                    if let Some(sys) = system {
                        prompt.push_str(sys);
                        prompt.push('\n');
                    }
                    prompt.push_str(&msg.content);
                    prompt.push_str(" [/INST]");
                }
                MistralRole::Assistant => {
                    prompt.push(' ');
                    prompt.push_str(&msg.content);
                    prompt.push_str("</s>");
                }
            }
            i += 1;
        }

        prompt
    }
}

impl MistralResponse {
    /// Get the first output text.
    pub fn text(&self) -> String {
        self.outputs
            .first()
            .map(|o| o.text.clone())
            .unwrap_or_default()
    }
}

/// Mistral message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MistralMessage {
    /// The role.
    pub role: MistralRole,
    /// The content.
    pub content: String,
}

/// Mistral role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MistralRole {
    /// User message.
    User,
    /// Assistant message.
    Assistant,
}

impl MistralMessage {
    /// Create a new message.
    pub fn new(role: MistralRole, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
        }
    }

    /// Create a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self::new(MistralRole::User, content)
    }

    /// Create an assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(MistralRole::Assistant, content)
    }
}
