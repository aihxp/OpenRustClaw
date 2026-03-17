//! Embeddings API for Mistral AI.

use crate::client::MistralClient;
use crate::client::endpoints;
use crate::error::Result;
use crate::types::EmbeddingsResponse;

/// Client for the embeddings API.
#[derive(Debug)]
pub struct Embeddings<'a> {
    client: &'a MistralClient,
}

impl<'a> Embeddings<'a> {
    /// Create a new embeddings client.
    pub fn new(client: &'a MistralClient) -> Self {
        Self { client }
    }

    /// Create embeddings for a single input or multiple inputs.
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
    pub async fn embed(
        &self,
        model: impl Into<String>,
        input: impl Into<String>,
    ) -> Result<Vec<f32>> {
        let request = EmbeddingRequest::single(model, input);
        let response = self.create(request).await?;
        response
            .first()
            .map(|e| e.embedding.clone())
            .ok_or_else(|| crate::error::MistralError::Internal {
                message: "No embedding returned".to_string(),
            })
    }
}

/// An embedding request.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EmbeddingRequest {
    /// ID of the model to use (e.g., "mistral-embed").
    pub model: String,
    /// Input text to embed.
    pub input: EmbeddingInput,
    /// The format to return the embeddings in. Can be "float" or "base64".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding_format: Option<String>,
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
        }
    }

    /// Create a new embedding request for multiple inputs.
    pub fn new(model: impl Into<String>, inputs: Vec<String>) -> Self {
        Self {
            model: model.into(),
            input: EmbeddingInput::Multiple(inputs),
            encoding_format: None,
        }
    }

    /// Set the encoding format.
    ///
    /// # Arguments
    ///
    /// * `format` - Either "float" or "base64"
    pub fn encoding_format(mut self, format: impl Into<String>) -> Self {
        self.encoding_format = Some(format.into());
        self
    }

    /// Use float encoding format (default).
    pub fn float_format(mut self) -> Self {
        self.encoding_format = Some("float".to_string());
        self
    }

    /// Use base64 encoding format.
    pub fn base64_format(mut self) -> Self {
        self.encoding_format = Some("base64".to_string());
        self
    }
}

/// Builder for embedding requests.
#[derive(Debug, Clone)]
pub struct EmbeddingRequestBuilder {
    model: String,
    inputs: Vec<String>,
    encoding_format: Option<String>,
}

impl EmbeddingRequestBuilder {
    /// Create a new embedding request builder.
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            inputs: Vec::new(),
            encoding_format: None,
        }
    }

    /// Add a single input.
    pub fn input(mut self, input: impl Into<String>) -> Self {
        self.inputs.push(input.into());
        self
    }

    /// Add multiple inputs.
    pub fn inputs(mut self, inputs: Vec<String>) -> Self {
        self.inputs.extend(inputs);
        self
    }

    /// Set the encoding format.
    pub fn encoding_format(mut self, format: impl Into<String>) -> Self {
        self.encoding_format = Some(format.into());
        self
    }

    /// Use float encoding format (default).
    pub fn float_format(mut self) -> Self {
        self.encoding_format = Some("float".to_string());
        self
    }

    /// Use base64 encoding format.
    pub fn base64_format(mut self) -> Self {
        self.encoding_format = Some("base64".to_string());
        self
    }

    /// Build the request.
    pub fn build(self) -> EmbeddingRequest {
        let input = if self.inputs.len() == 1 {
            EmbeddingInput::Single(self.inputs.into_iter().next().unwrap())
        } else {
            EmbeddingInput::Multiple(self.inputs)
        };

        EmbeddingRequest {
            model: self.model,
            input,
            encoding_format: self.encoding_format,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_request_single() {
        let req = EmbeddingRequest::single("mistral-embed", "Hello world");
        match req.input {
            EmbeddingInput::Single(s) => assert_eq!(s, "Hello world"),
            _ => panic!("Expected single input"),
        }
    }

    #[test]
    fn test_embedding_request_multiple() {
        let inputs = vec!["Hello".to_string(), "World".to_string()];
        let req = EmbeddingRequest::new("mistral-embed", inputs);
        match req.input {
            EmbeddingInput::Multiple(v) => assert_eq!(v.len(), 2),
            _ => panic!("Expected multiple inputs"),
        }
    }

    #[test]
    fn test_embedding_request_builder() {
        let req = EmbeddingRequestBuilder::new("mistral-embed")
            .input("Hello")
            .input("World")
            .float_format()
            .build();

        assert_eq!(req.model, "mistral-embed");
        assert_eq!(req.encoding_format, Some("float".to_string()));
        match req.input {
            EmbeddingInput::Multiple(v) => assert_eq!(v.len(), 2),
            _ => panic!("Expected multiple inputs"),
        }
    }

    #[test]
    fn test_embedding_request_builder_single() {
        let req = EmbeddingRequestBuilder::new("mistral-embed")
            .input("Hello")
            .build();

        match req.input {
            EmbeddingInput::Single(s) => assert_eq!(s, "Hello"),
            _ => panic!("Expected single input"),
        }
    }
}
