//! Embeddings API.

use crate::client::OpenAIClient;
use crate::constants::endpoints;
use crate::error::Result;
use crate::types::{Embedding, EmbeddingsResponse};

/// Client for the embeddings API.
#[derive(Debug)]
pub struct Embeddings<'a> {
    client: &'a OpenAIClient,
}

impl<'a> Embeddings<'a> {
    /// Create a new embeddings client.
    pub fn new(client: &'a OpenAIClient) -> Self {
        Self { client }
    }

    /// Create embeddings for a single input.
    pub async fn create(&self, request: EmbeddingRequest) -> Result<EmbeddingsResponse> {
        let body = serde_json::to_value(&request)?;

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let body = body.clone();
                Box::pin(async move {
                    let response = client.post(endpoints::EMBEDDINGS, body).await?;
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
        model: impl Into<String>,
        inputs: Vec<String>,
    ) -> Result<Vec<Vec<f32>>> {
        let request = EmbeddingRequest::new(model, inputs);
        let response = self.create(request).await?;
        Ok(response.embeddings())
    }

    /// Create a single embedding.
    pub async fn embed(&self, model: impl Into<String>, input: impl Into<String>) -> Result<Vec<f32>> {
        let request = EmbeddingRequest::single(model, input);
        let response = self.create(request).await?;
        response
            .first()
            .map(|e| e.embedding.clone())
            .ok_or_else(|| crate::error::OpenAIError::Internal {
                message: "No embedding returned".to_string(),
            })
    }
}

/// An embedding request.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EmbeddingRequest {
    /// ID of the model to use.
    pub model: String,

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
}

impl EmbeddingRequest {
    /// Create a new embedding request for a single input.
    pub fn single(model: impl Into<String>, input: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            input: EmbeddingInput::Single(input.into()),
            encoding_format: None,
            dimensions: None,
            user: None,
        }
    }

    /// Create a new embedding request for multiple inputs.
    pub fn new(model: impl Into<String>, inputs: Vec<String>) -> Self {
        Self {
            model: model.into(),
            input: EmbeddingInput::Multiple(inputs),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_request_single() {
        let req = EmbeddingRequest::single("text-embedding-3-small", "Hello world");
        match req.input {
            EmbeddingInput::Single(s) => assert_eq!(s, "Hello world"),
            _ => panic!("Expected single input"),
        }
    }

    #[test]
    fn test_embedding_request_multiple() {
        let inputs = vec!["Hello".to_string(), "World".to_string()];
        let req = EmbeddingRequest::new("text-embedding-3-small", inputs);
        match req.input {
            EmbeddingInput::Multiple(v) => assert_eq!(v.len(), 2),
            _ => panic!("Expected multiple inputs"),
        }
    }
}
