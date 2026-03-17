//! Embeddings API for Azure OpenAI.

use crate::client::AzureOpenAIClient;
use crate::error::Result;
use crate::types::{Embedding, EmbeddingsResponse};

/// Client for the embeddings API.
#[derive(Debug)]
pub struct Embeddings<'a> {
    client: &'a AzureOpenAIClient,
}

impl<'a> Embeddings<'a> {
    /// Create a new embeddings client.
    pub fn new(client: &'a AzureOpenAIClient) -> Self {
        Self { client }
    }

    /// Create embeddings for inputs.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use azure_openai::{AzureOpenAIClient, EmbeddingRequest};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = AzureOpenAIClient::new(
    ///     "my-resource",
    ///     "my-deployment",
    ///     "my-api-key",
    /// )?;
    ///
    /// let request = EmbeddingRequest::single("Hello, world!");
    /// let response = client.embeddings().create(request).await?;
    ///
    /// if let Some(embedding) = response.first() {
    ///     println!("Dimensions: {}", embedding.dimensions());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(&self, request: EmbeddingRequest) -> Result<EmbeddingsResponse> {
        let body = serde_json::to_value(&request)?;
        let deployment = self.client.deployment_name().to_string();

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let body = body.clone();
                let path = format!("/openai/deployments/{}/embeddings", &deployment);
                Box::pin(async move {
                    let response = client.post(&path, body).await?;
                    let body = client.handle_response(response).await?;
                    let embedding_response: EmbeddingsResponse = serde_json::from_value(body)?;
                    Ok(embedding_response)
                })
            })
            .await
    }

    /// Create embeddings for multiple inputs.
    pub async fn create_many(
        &self,
        inputs: Vec<String>,
    ) -> Result<Vec<Vec<f32>>> {
        let request = EmbeddingRequest::new(inputs);
        let response = self.create(request).await?;
        Ok(response.embeddings())
    }

    /// Create a single embedding.
    pub async fn embed(&self, input: impl Into<String>) -> Result<Vec<f32>> {
        let request = EmbeddingRequest::single(input);
        let response = self.create(request).await?;
        response
            .first()
            .map(|e| e.embedding.clone())
            .ok_or_else(|| crate::error::AzureOpenAIError::Internal {
                message: "No embedding returned".to_string(),
            })
    }

    /// Create embeddings for multiple texts with dimensions parameter.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use azure_openai::{AzureOpenAIClient, EmbeddingRequest};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = AzureOpenAIClient::new(
    ///     "my-resource",
    ///     "my-deployment",
    ///     "my-api-key",
    /// )?;
    ///
    /// let inputs = vec![
    ///     "First text to embed".to_string(),
    ///     "Second text to embed".to_string(),
    /// ];
    ///
    /// let request = EmbeddingRequest::new(inputs).dimensions(512);
    /// let response = client.embeddings().create(request).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_with_dimensions(
        &self,
        inputs: Vec<String>,
        dimensions: usize,
    ) -> Result<EmbeddingsResponse> {
        let request = EmbeddingRequest::new(inputs).dimensions(dimensions);
        self.create(request).await
    }
}

/// An embedding request.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EmbeddingRequest {
    /// Input text to embed.
    pub input: EmbeddingInput,

    /// The format to return the embeddings in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding_format: Option<String>,

    /// The number of dimensions the resulting output embeddings should have.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<usize>,

    /// User identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

/// Input for embedding requests.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum EmbeddingInput {
    /// Single string input.
    Single(String),
    /// Multiple string inputs.
    Multiple(Vec<String>),
    /// Token array input (for advanced use cases).
    Tokens(Vec<i32>),
    /// Multiple token arrays.
    MultipleTokens(Vec<Vec<i32>>),
}

impl EmbeddingRequest {
    /// Create a new embedding request for a single input.
    pub fn single(input: impl Into<String>) -> Self {
        Self {
            input: EmbeddingInput::Single(input.into()),
            encoding_format: None,
            dimensions: None,
            user: None,
        }
    }

    /// Create a new embedding request for multiple inputs.
    pub fn new(inputs: Vec<String>) -> Self {
        Self {
            input: EmbeddingInput::Multiple(inputs),
            encoding_format: None,
            dimensions: None,
            user: None,
        }
    }

    /// Create a new embedding request from token IDs.
    pub fn from_tokens(tokens: Vec<i32>) -> Self {
        Self {
            input: EmbeddingInput::Tokens(tokens),
            encoding_format: None,
            dimensions: None,
            user: None,
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

    /// Set the user.
    pub fn user(mut self, user: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self
    }
}

/// Extension trait for embedding operations.
pub trait EmbeddingExt {
    /// Calculate cosine similarity with another embedding.
    fn cosine_similarity(&self, other: &Self) -> f32;
    /// Normalize the embedding to unit length.
    fn normalize(&self) -> Vec<f32>;
}

impl EmbeddingExt for Embedding {
    fn cosine_similarity(&self, other: &Self) -> f32 {
        if self.embedding.len() != other.embedding.len() {
            return 0.0;
        }

        let dot_product: f32 = self
            .embedding
            .iter()
            .zip(other.embedding.iter())
            .map(|(a, b)| a * b)
            .sum();

        let norm_a: f32 = self.embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = other.embedding.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            0.0
        } else {
            dot_product / (norm_a * norm_b)
        }
    }

    fn normalize(&self) -> Vec<f32> {
        let norm: f32 = self.embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm == 0.0 {
            return self.embedding.clone();
        }
        self.embedding.iter().map(|x| x / norm).collect()
    }
}

impl EmbeddingExt for Vec<f32> {
    fn cosine_similarity(&self, other: &Self) -> f32 {
        if self.len() != other.len() {
            return 0.0;
        }

        let dot_product: f32 = self.iter().zip(other.iter()).map(|(a, b)| a * b).sum();
        let norm_a: f32 = self.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = other.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            0.0
        } else {
            dot_product / (norm_a * norm_b)
        }
    }

    fn normalize(&self) -> Vec<f32> {
        let norm: f32 = self.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm == 0.0 {
            return self.clone();
        }
        self.iter().map(|x| x / norm).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_request_single() {
        let req = EmbeddingRequest::single("Hello world");
        match req.input {
            EmbeddingInput::Single(s) => assert_eq!(s, "Hello world"),
            _ => panic!("Expected single input"),
        }
    }

    #[test]
    fn test_embedding_request_multiple() {
        let inputs = vec!["Hello".to_string(), "World".to_string()];
        let req = EmbeddingRequest::new(inputs);
        match req.input {
            EmbeddingInput::Multiple(v) => assert_eq!(v.len(), 2),
            _ => panic!("Expected multiple inputs"),
        }
    }

    #[test]
    fn test_embedding_dimensions() {
        let req = EmbeddingRequest::single("Hello").dimensions(512);
        assert_eq!(req.dimensions, Some(512));
    }

    #[test]
    fn test_cosine_similarity() {
        let embedding1 = Embedding {
            object: "embedding".to_string(),
            embedding: vec![1.0, 0.0, 0.0],
            index: 0,
        };
        let embedding2 = Embedding {
            object: "embedding".to_string(),
            embedding: vec![1.0, 0.0, 0.0],
            index: 1,
        };
        let embedding3 = Embedding {
            object: "embedding".to_string(),
            embedding: vec![0.0, 1.0, 0.0],
            index: 2,
        };

        // Same vectors should have similarity 1.0
        assert!((embedding1.cosine_similarity(&embedding2) - 1.0).abs() < 0.0001);

        // Orthogonal vectors should have similarity 0.0
        assert!(embedding1.cosine_similarity(&embedding3).abs() < 0.0001);
    }

    #[test]
    fn test_normalize() {
        let embedding = Embedding {
            object: "embedding".to_string(),
            embedding: vec![3.0, 4.0],
            index: 0,
        };

        let normalized = embedding.normalize();
        let expected_norm = 1.0;
        let actual_norm: f32 = normalized.iter().map(|x| x * x).sum::<f32>().sqrt();

        assert!((actual_norm - expected_norm).abs() < 0.0001);
    }
}
