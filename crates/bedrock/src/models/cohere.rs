//! Cohere model-specific types.

use serde::{Deserialize, Serialize};

/// Cohere Command R/R+ request body.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CohereCommandRRequest {
    /// The message.
    pub message: String,
    /// Chat history.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chat_history: Option<Vec<CohereMessage>>,
    /// Documents for RAG.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documents: Option<Vec<CohereDocument>>,
    /// Temperature.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Top-p.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub p: Option<f32>,
    /// Top-k.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub k: Option<i32>,
    /// Maximum number of tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i32>,
    /// Stop sequences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    /// Stream response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

/// Cohere message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohereMessage {
    /// The role.
    pub role: CohereRole,
    /// The message.
    pub message: String,
}

/// Cohere role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum CohereRole {
    /// User message.
    User,
    /// Chatbot message.
    Chatbot,
    /// System message.
    System,
}

/// Cohere document for RAG.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohereDocument {
    /// Document ID.
    pub id: String,
    /// Document text.
    pub text: String,
    /// Additional metadata.
    #[serde(flatten)]
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

/// Cohere Command R/R+ response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CohereCommandRResponse {
    /// The response text.
    pub text: String,
    /// Generation ID.
    pub generation_id: String,
    /// Finish reason.
    pub finish_reason: String,
    /// Document citations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citations: Option<Vec<CohereCitation>>,
    /// Documents referenced.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documents: Option<Vec<CohereDocument>>,
    /// Search queries generated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_queries: Option<Vec<CohereSearchQuery>>,
    /// Token count information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<CohereMeta>,
}

/// Cohere citation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CohereCitation {
    /// Start position.
    pub start: i32,
    /// End position.
    pub end: i32,
    /// Cited text.
    pub text: String,
    /// Document IDs.
    pub document_ids: Vec<String>,
}

/// Cohere search query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohereSearchQuery {
    /// The search query text.
    pub text: String,
    /// Generation ID.
    pub generation_id: String,
}

/// Cohere meta information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohereMeta {
    /// Token count.
    pub tokens: CohereTokens,
}

/// Cohere token counts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohereTokens {
    /// Input tokens.
    pub input_tokens: i32,
    /// Output tokens.
    pub output_tokens: i32,
}

/// Cohere Command (legacy) request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohereCommandRequest {
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
    pub p: Option<f32>,
    /// Top-k.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub k: Option<i32>,
    /// Return likelihoods.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_likelihoods: Option<String>,
    /// Stop sequences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    /// Stream.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    /// Num generations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_generations: Option<i32>,
}

/// Cohere Command (legacy) response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohereCommandResponse {
    /// The generations.
    pub generations: Vec<CohereGeneration>,
}

/// Cohere generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohereGeneration {
    /// The generated text.
    pub text: String,
    /// Generation ID.
    pub id: String,
    /// Finish reason.
    pub finish_reason: String,
}

/// Cohere embedding request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohereEmbedRequest {
    /// The texts to embed.
    pub texts: Vec<String>,
    /// The input type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_type: Option<String>,
    /// Truncate mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncate: Option<String>,
}

/// Cohere embedding response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CohereEmbedResponse {
    /// The embeddings.
    pub embeddings: Vec<Vec<f32>>,
    /// Texts.
    pub texts: Vec<String>,
    /// Meta information.
    pub meta: CohereMeta,
}
