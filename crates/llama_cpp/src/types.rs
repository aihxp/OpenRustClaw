//! Type definitions for the llama.cpp API.

use serde::{Deserialize, Serialize};

/// The role of a message author.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// System message.
    System,
    /// User message.
    User,
    /// Assistant message.
    Assistant,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::System => write!(f, "system"),
            Role::User => write!(f, "user"),
            Role::Assistant => write!(f, "assistant"),
        }
    }
}

/// A chat message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// The role of the message author.
    pub role: Role,
    /// The content of the message.
    pub content: String,
}

impl ChatMessage {
    /// Create a new message with the given role and content.
    pub fn new(role: Role, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
        }
    }

    /// Create a system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self::new(Role::System, content)
    }

    /// Create a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self::new(Role::User, content)
    }

    /// Create an assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(Role::Assistant, content)
    }
}

/// A chat completion response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    /// Unique identifier for the completion.
    #[serde(rename = "id")]
    pub id: String,
    /// The object type.
    pub object: String,
    /// The Unix timestamp when the completion was created.
    pub created: i64,
    /// The model used for completion.
    pub model: String,
    /// The list of completion choices.
    pub choices: Vec<ChatChoice>,
    /// Usage statistics (optional, may not be present in all responses).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<TokenUsage>,
}

impl ChatResponse {
    /// Get the content of the first choice.
    pub fn content(&self) -> Option<&str> {
        self.choices.first().map(|c| c.message.content.as_str())
    }

    /// Get the message of the first choice.
    pub fn message(&self) -> Option<&ChatMessage> {
        self.choices.first().map(|c| &c.message)
    }

    /// Get the finish reason of the first choice.
    pub fn finish_reason(&self) -> Option<FinishReason> {
        self.choices.first().and_then(|c| c.finish_reason)
    }
}

/// A chat completion choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChoice {
    /// The index of this choice.
    pub index: usize,
    /// The message.
    pub message: ChatMessage,
    /// The reason the completion finished.
    pub finish_reason: Option<FinishReason>,
}

/// Token usage statistics.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Tokens in the prompt.
    #[serde(rename = "prompt_tokens")]
    pub prompt_tokens: usize,
    /// Tokens in the completion.
    #[serde(rename = "completion_tokens")]
    pub completion_tokens: usize,
    /// Total tokens.
    #[serde(rename = "total_tokens")]
    pub total_tokens: usize,
}

impl TokenUsage {
    /// Create new usage statistics.
    pub fn new(prompt: usize, completion: usize) -> Self {
        Self {
            prompt_tokens: prompt,
            completion_tokens: completion,
            total_tokens: prompt + completion,
        }
    }

    /// Get the total tokens.
    pub fn total(&self) -> usize {
        self.total_tokens
    }
}

impl std::ops::Add for TokenUsage {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            prompt_tokens: self.prompt_tokens + other.prompt_tokens,
            completion_tokens: self.completion_tokens + other.completion_tokens,
            total_tokens: self.total_tokens + other.total_tokens,
        }
    }
}

/// The reason the model stopped generating.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    /// The model hit a natural stop point or a provided stop sequence.
    Stop,
    /// The maximum number of tokens specified in the request was reached.
    Length,
}

impl std::fmt::Display for FinishReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FinishReason::Stop => write!(f, "stop"),
            FinishReason::Length => write!(f, "length"),
        }
    }
}

/// A completion response (non-chat).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    /// Unique identifier for the completion.
    #[serde(rename = "id")]
    pub id: String,
    /// The object type.
    pub object: String,
    /// The Unix timestamp when the completion was created.
    pub created: i64,
    /// The model used for completion.
    pub model: String,
    /// The list of completion choices.
    pub choices: Vec<CompletionChoice>,
    /// Usage statistics.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<TokenUsage>,
}

impl CompletionResponse {
    /// Get the content of the first choice.
    pub fn content(&self) -> Option<&str> {
        self.choices.first().map(|c| c.text.as_str())
    }

    /// Get the finish reason of the first choice.
    pub fn finish_reason(&self) -> Option<FinishReason> {
        self.choices.first().and_then(|c| c.finish_reason)
    }
}

/// A completion choice (non-chat).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionChoice {
    /// The index of this choice.
    pub index: usize,
    /// The generated text.
    pub text: String,
    /// Logprobs (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<serde_json::Value>,
    /// The reason the completion finished.
    pub finish_reason: Option<FinishReason>,
}

/// Tokenize response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenizeResponse {
    /// The list of token IDs.
    pub tokens: Vec<i32>,
}

/// Embedding response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResponse {
    /// The embedding vector.
    pub embedding: Vec<f32>,
}

/// Health response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Server status.
    pub status: String,
    /// Whether the server is in a slot save emergency.
    #[serde(rename = "slots_idle", default)]
    pub slots_idle: Option<usize>,
    /// Number of processing slots.
    #[serde(rename = "slots_processing", default)]
    pub slots_processing: Option<usize>,
}

impl HealthResponse {
    /// Check if the server is healthy.
    pub fn is_healthy(&self) -> bool {
        self.status == "ok" || self.status == "healthy"
    }
}

/// Slot information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotInfo {
    /// Slot ID.
    pub id: usize,
    /// Current state of the slot.
    pub state: SlotState,
    /// Current prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// Generation parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

/// Slot state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SlotState {
    /// Slot is idle.
    Idle,
    /// Slot is processing.
    Processing,
}

/// Slots response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotsResponse {
    /// List of slots.
    pub slots: Vec<SlotInfo>,
}

/// Supported GGUF model architectures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GgufArchitecture {
    /// Llama 2/3 models.
    Llama,
    /// Mistral models.
    Mistral,
    /// Mixtral models.
    Mixtral,
    /// CodeLlama models.
    CodeLlama,
    /// Phi models.
    Phi,
    /// Qwen models.
    Qwen,
    /// Gemma models.
    Gemma,
    /// Other/custom models.
    Other,
}

impl GgufArchitecture {
    /// Get the architecture name.
    pub fn as_str(&self) -> &'static str {
        match self {
            GgufArchitecture::Llama => "llama",
            GgufArchitecture::Mistral => "mistral",
            GgufArchitecture::Mixtral => "mixtral",
            GgufArchitecture::CodeLlama => "codellama",
            GgufArchitecture::Phi => "phi",
            GgufArchitecture::Qwen => "qwen",
            GgufArchitecture::Gemma => "gemma",
            GgufArchitecture::Other => "other",
        }
    }
}

impl std::fmt::Display for GgufArchitecture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Sampling parameters for generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplingParams {
    /// Temperature for sampling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Nucleus sampling parameter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Top-k sampling parameter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i32>,
    /// Minimum probability for sampling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_p: Option<f32>,
    /// Repetition penalty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repeat_penalty: Option<f32>,
    /// Presence penalty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    /// Frequency penalty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    /// Number of tokens to consider for repetition penalty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub penalty_last_n: Option<i32>,
    /// Typical sampling parameter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub typical_p: Option<f32>,
    /// TFS (Tail Free Sampling) parameter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tfs_z: Option<f32>,
}

impl Default for SamplingParams {
    fn default() -> Self {
        Self {
            temperature: None,
            top_p: None,
            top_k: None,
            min_p: None,
            repeat_penalty: None,
            presence_penalty: None,
            frequency_penalty: None,
            penalty_last_n: None,
            typical_p: None,
            tfs_z: None,
        }
    }
}

impl SamplingParams {
    /// Create a new builder.
    pub fn builder() -> SamplingParamsBuilder {
        SamplingParamsBuilder::default()
    }
}

/// Builder for sampling parameters.
#[derive(Debug, Default, Clone)]
pub struct SamplingParamsBuilder {
    temperature: Option<f32>,
    top_p: Option<f32>,
    top_k: Option<i32>,
    min_p: Option<f32>,
    repeat_penalty: Option<f32>,
    presence_penalty: Option<f32>,
    frequency_penalty: Option<f32>,
    penalty_last_n: Option<i32>,
    typical_p: Option<f32>,
    tfs_z: Option<f32>,
}

impl SamplingParamsBuilder {
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
    pub fn top_k(mut self, top_k: i32) -> Self {
        self.top_k = Some(top_k.max(1));
        self
    }

    /// Set min-p.
    pub fn min_p(mut self, min_p: f32) -> Self {
        self.min_p = Some(min_p.clamp(0.0, 1.0));
        self
    }

    /// Set repeat penalty.
    pub fn repeat_penalty(mut self, penalty: f32) -> Self {
        self.repeat_penalty = Some(penalty);
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

    /// Set penalty last n.
    pub fn penalty_last_n(mut self, n: i32) -> Self {
        self.penalty_last_n = Some(n.max(0));
        self
    }

    /// Set typical p.
    pub fn typical_p(mut self, p: f32) -> Self {
        self.typical_p = Some(p.clamp(0.0, 1.0));
        self
    }

    /// Set tfs z.
    pub fn tfs_z(mut self, z: f32) -> Self {
        self.tfs_z = Some(z.max(0.0));
        self
    }

    /// Build the parameters.
    pub fn build(self) -> SamplingParams {
        SamplingParams {
            temperature: self.temperature,
            top_p: self.top_p,
            top_k: self.top_k,
            min_p: self.min_p,
            repeat_penalty: self.repeat_penalty,
            presence_penalty: self.presence_penalty,
            frequency_penalty: self.frequency_penalty,
            penalty_last_n: self.penalty_last_n,
            typical_p: self.typical_p,
            tfs_z: self.tfs_z,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = ChatMessage::user("Hello");
        assert_eq!(msg.role, Role::User);
        assert_eq!(msg.content, "Hello");
    }

    #[test]
    fn test_chat_response_helpers() {
        let response = ChatResponse {
            id: "chatcmpl-123".to_string(),
            object: "chat.completion".to_string(),
            created: 1234567890,
            model: "llama-3-8b".to_string(),
            choices: vec![ChatChoice {
                index: 0,
                message: ChatMessage::assistant("Hello!"),
                finish_reason: Some(FinishReason::Stop),
            }],
            usage: Some(TokenUsage::new(10, 5)),
        };

        assert_eq!(response.content(), Some("Hello!"));
    }

    #[test]
    fn test_completion_response_helpers() {
        let response = CompletionResponse {
            id: "cmpl-123".to_string(),
            object: "text_completion".to_string(),
            created: 1234567890,
            model: "llama-3-8b".to_string(),
            choices: vec![CompletionChoice {
                index: 0,
                text: "Hello world".to_string(),
                logprobs: None,
                finish_reason: Some(FinishReason::Stop),
            }],
            usage: Some(TokenUsage::new(5, 2)),
        };

        assert_eq!(response.content(), Some("Hello world"));
    }

    #[test]
    fn test_health_response() {
        let healthy = HealthResponse {
            status: "ok".to_string(),
            slots_idle: Some(2),
            slots_processing: Some(1),
        };
        assert!(healthy.is_healthy());

        let unhealthy = HealthResponse {
            status: "error".to_string(),
            slots_idle: None,
            slots_processing: None,
        };
        assert!(!unhealthy.is_healthy());
    }

    #[test]
    fn test_sampling_params_builder() {
        let params = SamplingParams::builder()
            .temperature(0.7)
            .top_p(0.9)
            .top_k(40)
            .build();

        assert_eq!(params.temperature, Some(0.7));
        assert_eq!(params.top_p, Some(0.9));
        assert_eq!(params.top_k, Some(40));
    }

    #[test]
    fn test_usage_math() {
        let u1 = TokenUsage::new(10, 5);
        let u2 = TokenUsage::new(8, 4);
        let total = u1 + u2;
        assert_eq!(total.prompt_tokens, 18);
        assert_eq!(total.completion_tokens, 9);
        assert_eq!(total.total(), 27);
    }

    #[test]
    fn test_finish_reason_display() {
        assert_eq!(FinishReason::Stop.to_string(), "stop");
        assert_eq!(FinishReason::Length.to_string(), "length");
    }
}
