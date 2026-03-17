//! Embeddings API for llama.cpp.

use crate::client::LlamaCppClient;
use crate::client::endpoints;
use crate::error::Result;
use crate::types::EmbeddingResponse;

/// Client for the embeddings API.
#[derive(Debug)]
pub struct Embeddings<'a> {
    client: &'a LlamaCppClient,
}

impl<'a> Embeddings<'a> {
    /// Create a new embeddings client.
    pub fn new(client: &'a LlamaCppClient) -> Self {
        Self { client }
    }

    /// Generate embeddings for text.
    pub async fn embed(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse> {
        let body = serde_json::to_value(&request)?;

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let body = body.clone();
                Box::pin(async move {
                    let response = client.post(endpoints::EMBEDDING, body).await?;
                    let body = client.handle_response(response).await?;
                    let embedding_response: EmbeddingResponse = serde_json::from_value(body)?;
                    Ok(embedding_response)
                })
            })
            .await
    }

    /// Convenience method to embed a simple string.
    pub async fn embed_text(&self, content: impl Into<String>) -> Result<EmbeddingResponse> {
        self.embed(EmbeddingRequest::new(content)).await
    }

    /// Embed multiple texts in batch.
    pub async fn embed_batch(&self, contents: Vec<String>) -> Result<Vec<EmbeddingResponse>> {
        let mut results = Vec::with_capacity(contents.len());
        for content in contents {
            results.push(self.embed_text(content).await?);
        }
        Ok(results)
    }
}

/// An embedding request.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EmbeddingRequest {
    /// The content to embed.
    pub content: String,
    /// Add special tokens (like BOS/EOS).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub add_special: Option<bool>,
    /// Normalize the embedding vector.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub normalize: Option<bool>,
    /// Truncation direction.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncation_direction: Option<TruncationDirection>,
}

impl EmbeddingRequest {
    /// Create a new embedding request.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            add_special: None,
            normalize: None,
            truncation_direction: None,
        }
    }

    /// Create a builder.
    pub fn builder(content: impl Into<String>) -> EmbeddingRequestBuilder {
        EmbeddingRequestBuilder::new(content)
    }

    /// Set whether to add special tokens.
    pub fn add_special(mut self, add: bool) -> Self {
        self.add_special = Some(add);
        self
    }

    /// Set whether to normalize the embedding.
    pub fn normalize(mut self, normalize: bool) -> Self {
        self.normalize = Some(normalize);
        self
    }

    /// Set the truncation direction.
    pub fn truncation_direction(mut self, direction: TruncationDirection) -> Self {
        self.truncation_direction = Some(direction);
        self
    }
}

/// Truncation direction for embeddings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TruncationDirection {
    /// Truncate from the beginning.
    Start,
    /// Truncate from the end.
    End,
}

impl std::fmt::Display for TruncationDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TruncationDirection::Start => write!(f, "start"),
            TruncationDirection::End => write!(f, "end"),
        }
    }
}

/// Builder for embedding requests.
#[derive(Debug, Clone)]
pub struct EmbeddingRequestBuilder {
    content: String,
    add_special: Option<bool>,
    normalize: Option<bool>,
    truncation_direction: Option<TruncationDirection>,
}

impl EmbeddingRequestBuilder {
    /// Create a new builder.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            add_special: None,
            normalize: None,
            truncation_direction: None,
        }
    }

    /// Set whether to add special tokens.
    pub fn add_special(mut self, add: bool) -> Self {
        self.add_special = Some(add);
        self
    }

    /// Set whether to normalize the embedding.
    pub fn normalize(mut self, normalize: bool) -> Self {
        self.normalize = Some(normalize);
        self
    }

    /// Set the truncation direction.
    pub fn truncation_direction(mut self, direction: TruncationDirection) -> Self {
        self.truncation_direction = Some(direction);
        self
    }

    /// Build the request.
    pub fn build(self) -> EmbeddingRequest {
        EmbeddingRequest {
            content: self.content,
            add_special: self.add_special,
            normalize: self.normalize,
            truncation_direction: self.truncation_direction,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_new() {
        let request = EmbeddingRequest::new("Hello world");
        assert_eq!(request.content, "Hello world");
        assert_eq!(request.add_special, None);
    }

    #[test]
    fn test_request_builder() {
        let request = EmbeddingRequest::builder("Hello world")
            .add_special(true)
            .normalize(true)
            .truncation_direction(TruncationDirection::End)
            .build();

        assert_eq!(request.content, "Hello world");
        assert_eq!(request.add_special, Some(true));
        assert_eq!(request.normalize, Some(true));
        assert_eq!(request.truncation_direction, Some(TruncationDirection::End));
    }

    #[test]
    fn test_request_methods() {
        let request = EmbeddingRequest::new("Hello")
            .add_special(true)
            .normalize(false)
            .truncation_direction(TruncationDirection::Start);

        assert_eq!(request.add_special, Some(true));
        assert_eq!(request.normalize, Some(false));
        assert_eq!(
            request.truncation_direction,
            Some(TruncationDirection::Start)
        );
    }

    #[test]
    fn test_truncation_direction_display() {
        assert_eq!(TruncationDirection::Start.to_string(), "start");
        assert_eq!(TruncationDirection::End.to_string(), "end");
    }
}
