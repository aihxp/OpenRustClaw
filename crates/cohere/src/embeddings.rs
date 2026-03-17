//! Embeddings API for Cohere's embedding models.

use serde::{Deserialize, Serialize};

use crate::client::CohereClient;
use crate::constants::endpoints;
use crate::error::Result;
use crate::types::ApiMeta;

/// Client for the Embeddings API.
#[derive(Debug)]
pub struct EmbedEndpoint<'a> {
    pub(crate) client: &'a CohereClient,
}

impl<'a> EmbedEndpoint<'a> {
    /// Send an embed request.
    pub async fn create(&self, request: EmbedRequest) -> Result<EmbedResponse> {
        let body = serde_json::to_value(&request)?;

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let body = body.clone();
                Box::pin(async move {
                    let response = client.post(endpoints::EMBED, body).await?;
                    let body = client.handle_response(response).await?;
                    let embed_response: EmbedResponse = serde_json::from_value(body)?;
                    Ok(embed_response)
                })
            })
            .await
    }
}

/// A request to the Cohere Embeddings API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedRequest {
    /// The texts to embed.
    pub texts: Vec<String>,

    /// The model to use (e.g., "embed-english-v3.0").
    pub model: String,

    /// The type of input (required for v3 models).
    #[serde(rename = "input_type", skip_serializing_if = "Option::is_none")]
    pub input_type: Option<InputType>,

    /// Truncate the input if it's too long.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncate: Option<TruncateMode>,

    /// Embedding types to return.
    #[serde(rename = "embedding_types", skip_serializing_if = "Option::is_none")]
    pub embedding_types: Option<Vec<EmbeddingType>>,
}

impl EmbedRequest {
    /// Create a new request builder.
    pub fn builder() -> EmbedRequestBuilder {
        EmbedRequestBuilder::new()
    }

    /// Create a simple request for a single text.
    pub fn simple(model: impl Into<String>, text: impl Into<String>) -> Self {
        Self::builder().model(model).add_text(text).build()
    }

    /// Create a request for multiple texts.
    pub fn batch(model: impl Into<String>, texts: Vec<String>) -> Self {
        Self::builder().model(model).texts(texts).build()
    }
}

/// Builder for embed requests.
#[derive(Debug, Clone)]
pub struct EmbedRequestBuilder {
    texts: Vec<String>,
    model: Option<String>,
    input_type: Option<InputType>,
    truncate: Option<TruncateMode>,
    embedding_types: Option<Vec<EmbeddingType>>,
}

impl EmbedRequestBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            texts: Vec::new(),
            model: None,
            input_type: None,
            truncate: None,
            embedding_types: None,
        }
    }

    /// Set the model.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Add a text to embed.
    pub fn add_text(mut self, text: impl Into<String>) -> Self {
        self.texts.push(text.into());
        self
    }

    /// Set the texts to embed.
    pub fn texts(mut self, texts: Vec<String>) -> Self {
        self.texts = texts;
        self
    }

    /// Set the input type.
    pub fn input_type(mut self, input_type: InputType) -> Self {
        self.input_type = Some(input_type);
        self
    }

    /// Set the truncation mode.
    pub fn truncate(mut self, mode: TruncateMode) -> Self {
        self.truncate = Some(mode);
        self
    }

    /// Set the embedding types.
    pub fn embedding_types(mut self, types: Vec<EmbeddingType>) -> Self {
        self.embedding_types = Some(types);
        self
    }

    /// Build the request.
    pub fn build(self) -> EmbedRequest {
        EmbedRequest {
            texts: self.texts,
            model: self
                .model
                .unwrap_or_else(|| "embed-english-v3.0".to_string()),
            input_type: self.input_type,
            truncate: self.truncate,
            embedding_types: self.embedding_types,
        }
    }
}

impl Default for EmbedRequestBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Input type for embeddings (required for v3 models).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputType {
    /// For embeddings of search queries.
    SearchQuery,
    /// For embeddings of documents to search over.
    SearchDocument,
    /// For embeddings of text for classification.
    Classification,
    /// For embeddings of text for clustering.
    Clustering,
}

/// Truncation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TruncateMode {
    /// Truncate only the start.
    Start,
    /// Truncate only the end.
    End,
    /// Truncate both start and end.
    Both,
    /// Don't truncate (error if too long).
    None,
}

/// Embedding type to return.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmbeddingType {
    /// Return the float embedding.
    Float,
    /// Return the int8 embedding.
    Int8,
    /// Return the uint8 embedding.
    Uint8,
    /// Return the binary embedding.
    Binary,
    /// Return the ubinary embedding.
    Ubinary,
}

/// A response from the Cohere Embeddings API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedResponse {
    /// The ID of the response.
    pub id: String,

    /// The text embeddings.
    pub embeddings: Option<Vec<Vec<f32>>>,

    /// Embeddings by type (if multiple types requested).
    #[serde(rename = "embeddings_by_type", skip_serializing_if = "Option::is_none")]
    pub embeddings_by_type: Option<serde_json::Value>,

    /// The texts that were embedded.
    pub texts: Vec<String>,

    /// API metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<ApiMeta>,
}

impl EmbedResponse {
    /// Get the embeddings.
    pub fn get_embeddings(&self) -> &[Vec<f32>] {
        self.embeddings.as_deref().unwrap_or(&[])
    }

    /// Get the first embedding.
    pub fn first(&self) -> Option<&[f32]> {
        self.embeddings
            .as_ref()
            .and_then(|e| e.first().map(|v| v.as_slice()))
    }

    /// Get the number of embeddings.
    pub fn len(&self) -> usize {
        self.embeddings.as_ref().map(|e| e.len()).unwrap_or(0)
    }

    /// Check if there are no embeddings.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A single embedding with its text.
#[derive(Debug, Clone)]
pub struct Embedding {
    /// The text that was embedded.
    pub text: String,
    /// The embedding vector.
    pub vector: Vec<f32>,
}

impl Embedding {
    /// Create a new embedding.
    pub fn new(text: impl Into<String>, vector: Vec<f32>) -> Self {
        Self {
            text: text.into(),
            vector,
        }
    }

    /// Get the dimension of the embedding.
    pub fn dimension(&self) -> usize {
        self.vector.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_basic() {
        let request = EmbedRequest::builder()
            .model("embed-english-v3.0")
            .add_text("Hello world")
            .input_type(InputType::SearchDocument)
            .build();

        assert_eq!(request.model, "embed-english-v3.0");
        assert_eq!(request.texts.len(), 1);
        assert_eq!(request.input_type, Some(InputType::SearchDocument));
    }

    #[test]
    fn test_builder_batch() {
        let texts = vec!["text1".to_string(), "text2".to_string()];
        let request = EmbedRequest::builder()
            .model("embed-multilingual-v3.0")
            .texts(texts)
            .truncate(TruncateMode::End)
            .build();

        assert_eq!(request.texts.len(), 2);
        assert_eq!(request.truncate, Some(TruncateMode::End));
    }

    #[test]
    fn test_response_helpers() {
        let response = EmbedResponse {
            id: "emb_123".to_string(),
            embeddings: Some(vec![vec![0.1, 0.2, 0.3], vec![0.4, 0.5, 0.6]]),
            embeddings_by_type: None,
            texts: vec!["text1".to_string(), "text2".to_string()],
            meta: None,
        };

        assert_eq!(response.len(), 2);
        assert_eq!(response.first(), Some(vec![0.1, 0.2, 0.3].as_slice()));
    }

    #[test]
    fn test_embedding() {
        let embedding = Embedding::new("test", vec![0.1, 0.2, 0.3]);

        assert_eq!(embedding.text, "test");
        assert_eq!(embedding.dimension(), 3);
    }
}
