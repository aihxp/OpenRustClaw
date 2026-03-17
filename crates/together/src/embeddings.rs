//! Embeddings API for Together AI.

use crate::client::TogetherClient;
use crate::constants::endpoints;
use crate::error::Result;
use crate::types::{Embedding, EmbeddingUsage};

/// Client for embeddings.
#[derive(Debug)]
pub struct Embeddings<'a> {
    client: &'a TogetherClient,
}

impl<'a> Embeddings<'a> {
    /// Create a new embeddings client.
    pub fn new(client: &'a TogetherClient) -> Self {
        Self { client }
    }

    /// Create embeddings for the given input.
    pub async fn create(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse> {
        let body = serde_json::to_value(&request)?;

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let body = body.clone();
                Box::pin(async move {
                    let response = client.post(endpoints::EMBEDDINGS, body).await?;
                    let body = client.handle_response(response).await?;
                    let embedding_response: EmbeddingResponse = serde_json::from_value(body)?;
                    Ok(embedding_response)
                })
            })
            .await
    }
}

/// Input for embedding requests.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum EmbeddingInput {
    /// Single string input.
    Single(String),
    /// Multiple string inputs.
    Multiple(Vec<String>),
}

impl From<String> for EmbeddingInput {
    fn from(s: String) -> Self {
        EmbeddingInput::Single(s)
    }
}

impl From<&str> for EmbeddingInput {
    fn from(s: &str) -> Self {
        EmbeddingInput::Single(s.to_string())
    }
}

impl From<Vec<String>> for EmbeddingInput {
    fn from(v: Vec<String>) -> Self {
        EmbeddingInput::Multiple(v)
    }
}

impl From<Vec<&str>> for EmbeddingInput {
    fn from(v: Vec<&str>) -> Self {
        EmbeddingInput::Multiple(v.into_iter().map(String::from).collect())
    }
}

/// An embedding request.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EmbeddingRequest {
    /// Model ID.
    pub model: String,
    /// Input text(s) to embed.
    pub input: EmbeddingInput,
    /// Encoding format (default: float).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding_format: Option<String>,
    /// Number of dimensions (for supported models).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<usize>,
}

impl EmbeddingRequest {
    /// Create a new embedding request.
    pub fn new(model: impl Into<String>, input: impl Into<EmbeddingInput>) -> Self {
        Self {
            model: model.into(),
            input: input.into(),
            encoding_format: None,
            dimensions: None,
        }
    }

    /// Set the encoding format.
    pub fn encoding_format(mut self, format: impl Into<String>) -> Self {
        self.encoding_format = Some(format.into());
        self
    }

    /// Set the number of dimensions.
    pub fn dimensions(mut self, dims: usize) -> Self {
        self.dimensions = Some(dims);
        self
    }
}

/// An embedding response.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EmbeddingResponse {
    /// The object type.
    pub object: String,
    /// The embedding data.
    pub data: Vec<Embedding>,
    /// The model used.
    pub model: String,
    /// Usage information.
    pub usage: EmbeddingUsage,
}

impl EmbeddingResponse {
    /// Get the first embedding.
    pub fn first(&self) -> Option<&Embedding> {
        self.data.first()
    }

    /// Get the embedding vector of the first result.
    pub fn first_embedding(&self) -> Option<&Vec<f32>> {
        self.first().map(|e| &e.embedding)
    }

    /// Get all embeddings as a vector of vectors.
    pub fn all_embeddings(&self) -> Vec<&Vec<f32>> {
        self.data.iter().map(|e| &e.embedding).collect()
    }
}

/// Helper function to calculate cosine similarity between two embeddings.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot_product / (norm_a * norm_b)
}

/// Helper function to calculate dot product between two embeddings.
pub fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// Helper function to normalize an embedding vector in-place.
pub fn normalize(embedding: &mut [f32]) {
    let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in embedding.iter_mut() {
            *x /= norm;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-6);

        let c = vec![0.0, 1.0, 0.0];
        assert!((cosine_similarity(&a, &c)).abs() < 1e-6);
    }

    #[test]
    fn test_dot_product() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![4.0, 5.0, 6.0];
        assert!((dot_product(&a, &b) - 32.0).abs() < 1e-6);
    }

    #[test]
    fn test_normalize() {
        let mut v = vec![3.0, 4.0];
        normalize(&mut v);
        let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6);
    }
}
