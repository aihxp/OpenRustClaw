//! Embeddings API for Cloudflare Workers AI.

use crate::client::CloudflareAiClient;
use crate::error::Result;
use crate::types::EmbeddingsResponse;

/// Client for the embeddings API.
#[derive(Debug)]
pub struct Embeddings<'a> {
    client: &'a CloudflareAiClient,
}

impl<'a> Embeddings<'a> {
    /// Create a new embeddings client.
    pub fn new(client: &'a CloudflareAiClient) -> Self {
        Self { client }
    }

    /// Create embeddings for a single input or multiple inputs.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use cloudflare_ai::{CloudflareAiClient, EmbeddingsRequest};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = CloudflareAiClient::new("your-account-id", "your-api-token")?;
    ///
    /// let request = EmbeddingsRequest::new("@cf/baai/bge-base-en-v1.5", "Hello world");
    /// let response = client.embeddings().create(request).await?;
    ///
    /// if let Some(embedding) = response.first() {
    ///     println!("Embedding dimension: {}", embedding.embedding.len());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(&self, request: EmbeddingsRequest) -> Result<EmbeddingsResponse> {
        let model = request.model.clone();
        let body = serde_json::to_value(&request.body)?;

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let body = body.clone();
                let model = model.clone();
                Box::pin(async move {
                    let response = client.post(&model, body).await?;
                    let result: EmbeddingsResponse = client.handle_response(response).await?;
                    Ok(result)
                })
            })
            .await
    }

    /// Create embeddings for a single text input.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use cloudflare_ai::CloudflareAiClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = CloudflareAiClient::new("your-account-id", "your-api-token")?;
    ///
    /// let embedding = client.embeddings()
    ///     .embed("@cf/baai/bge-base-en-v1.5", "Hello world")
    ///     .await?;
    /// println!("Embedding dimension: {}", embedding.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn embed(
        &self,
        model: impl Into<String>,
        input: impl Into<String>,
    ) -> Result<Vec<f32>> {
        let request = EmbeddingsRequest::new(model, input);
        let response = self.create(request).await?;
        response
            .first()
            .map(|e| e.embedding.clone())
            .ok_or_else(|| crate::error::CloudflareAiError::Internal {
                message: "No embedding returned".to_string(),
            })
    }

    /// Create embeddings for multiple text inputs.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use cloudflare_ai::CloudflareAiClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = CloudflareAiClient::new("your-account-id", "your-api-token")?;
    ///
    /// let texts = vec!["Hello world".to_string(), "Goodbye world".to_string()];
    /// let embeddings = client.embeddings()
    ///     .embed_many("@cf/baai/bge-base-en-v1.5", texts)
    ///     .await?;
    /// println!("Generated {} embeddings", embeddings.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn embed_many(
        &self,
        model: impl Into<String>,
        inputs: Vec<String>,
    ) -> Result<Vec<Vec<f32>>> {
        let request = EmbeddingsRequest::new_multi(model, inputs);
        let response = self.create(request).await?;
        Ok(response.embeddings())
    }
}

/// The body of an embeddings request.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum EmbeddingsBody {
    /// Single text input.
    Single {
        /// The text to embed.
        text: String,
    },
    /// Multiple text inputs.
    Multiple {
        /// The texts to embed.
        texts: Vec<String>,
    },
}

/// An embeddings request.
#[derive(Debug, Clone)]
pub struct EmbeddingsRequest {
    /// ID of the model to use (e.g., "@cf/baai/bge-base-en-v1.5").
    pub model: String,
    /// The body of the request.
    pub body: EmbeddingsBody,
}

impl EmbeddingsRequest {
    /// Create a new embeddings request for a single input.
    ///
    /// # Arguments
    ///
    /// * `model` - The model identifier (e.g., "@cf/baai/bge-base-en-v1.5")
    /// * `input` - The text to embed
    ///
    /// # Example
    ///
    /// ```
    /// use cloudflare_ai::EmbeddingsRequest;
    ///
    /// let request = EmbeddingsRequest::new("@cf/baai/bge-base-en-v1.5", "Hello world");
    /// ```
    pub fn new(model: impl Into<String>, input: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            body: EmbeddingsBody::Single { text: input.into() },
        }
    }

    /// Create a new embeddings request for multiple inputs.
    ///
    /// # Arguments
    ///
    /// * `model` - The model identifier (e.g., "@cf/baai/bge-base-en-v1.5")
    /// * `inputs` - The texts to embed
    ///
    /// # Example
    ///
    /// ```
    /// use cloudflare_ai::EmbeddingsRequest;
    ///
    /// let inputs = vec!["Hello".to_string(), "World".to_string()];
    /// let request = EmbeddingsRequest::new_multi("@cf/baai/bge-base-en-v1.5", inputs);
    /// ```
    pub fn new_multi(model: impl Into<String>, inputs: Vec<String>) -> Self {
        Self {
            model: model.into(),
            body: EmbeddingsBody::Multiple { texts: inputs },
        }
    }

    /// Create a builder for embeddings requests.
    pub fn builder(model: impl Into<String>) -> EmbeddingsRequestBuilder {
        EmbeddingsRequestBuilder::new(model)
    }
}

/// Builder for embeddings requests.
#[derive(Debug, Clone)]
pub struct EmbeddingsRequestBuilder {
    model: String,
    inputs: Vec<String>,
}

impl EmbeddingsRequestBuilder {
    /// Create a new embeddings request builder.
    ///
    /// # Arguments
    ///
    /// * `model` - The model identifier (e.g., "@cf/baai/bge-base-en-v1.5")
    ///
    /// # Example
    ///
    /// ```
    /// use cloudflare_ai::EmbeddingsRequest;
    ///
    /// let builder = EmbeddingsRequest::builder("@cf/baai/bge-base-en-v1.5");
    /// ```
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            inputs: Vec::new(),
        }
    }

    /// Add a single input.
    ///
    /// # Example
    ///
    /// ```
    /// use cloudflare_ai::EmbeddingsRequest;
    ///
    /// let request = EmbeddingsRequest::builder("@cf/baai/bge-base-en-v1.5")
    ///     .input("Hello")
    ///     .input("World")
    ///     .build();
    /// ```
    pub fn input(mut self, input: impl Into<String>) -> Self {
        self.inputs.push(input.into());
        self
    }

    /// Add multiple inputs.
    ///
    /// # Example
    ///
    /// ```
    /// use cloudflare_ai::EmbeddingsRequest;
    ///
    /// let inputs = vec!["Hello".to_string(), "World".to_string()];
    /// let request = EmbeddingsRequest::builder("@cf/baai/bge-base-en-v1.5")
    ///     .inputs(inputs)
    ///     .build();
    /// ```
    pub fn inputs(mut self, inputs: Vec<String>) -> Self {
        self.inputs.extend(inputs);
        self
    }

    /// Build the request.
    pub fn build(self) -> EmbeddingsRequest {
        if self.inputs.len() == 1 {
            EmbeddingsRequest {
                model: self.model,
                body: EmbeddingsBody::Single {
                    text: self.inputs.into_iter().next().unwrap(),
                },
            }
        } else {
            EmbeddingsRequest {
                model: self.model,
                body: EmbeddingsBody::Multiple { texts: self.inputs },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embeddings_request_single() {
        let req = EmbeddingsRequest::new("@cf/baai/bge-base-en-v1.5", "Hello world");
        match req.body {
            EmbeddingsBody::Single { text } => assert_eq!(text, "Hello world"),
            _ => panic!("Expected single input"),
        }
    }

    #[test]
    fn test_embeddings_request_multiple() {
        let inputs = vec!["Hello".to_string(), "World".to_string()];
        let req = EmbeddingsRequest::new_multi("@cf/baai/bge-base-en-v1.5", inputs);
        match req.body {
            EmbeddingsBody::Multiple { texts } => {
                assert_eq!(texts.len(), 2);
                assert_eq!(texts[0], "Hello");
                assert_eq!(texts[1], "World");
            }
            _ => panic!("Expected multiple inputs"),
        }
    }

    #[test]
    fn test_embeddings_request_builder_single() {
        let req = EmbeddingsRequest::builder("@cf/baai/bge-base-en-v1.5")
            .input("Hello")
            .build();

        assert_eq!(req.model, "@cf/baai/bge-base-en-v1.5");
        match req.body {
            EmbeddingsBody::Single { text } => assert_eq!(text, "Hello"),
            _ => panic!("Expected single input"),
        }
    }

    #[test]
    fn test_embeddings_request_builder_multiple() {
        let req = EmbeddingsRequest::builder("@cf/baai/bge-base-en-v1.5")
            .input("Hello")
            .input("World")
            .build();

        match req.body {
            EmbeddingsBody::Multiple { texts } => {
                assert_eq!(texts.len(), 2);
            }
            _ => panic!("Expected multiple inputs"),
        }
    }

    #[test]
    fn test_embeddings_response() {
        let response: EmbeddingsResponse = serde_json::from_str(
            r#"{
                "data": [
                    {"embedding": [0.1, 0.2, 0.3]},
                    {"embedding": [0.4, 0.5, 0.6]}
                ],
                "shape": [2, 3]
            }"#,
        )
        .unwrap();

        assert_eq!(response.data.len(), 2);
        assert_eq!(response.first().unwrap().embedding, vec![0.1, 0.2, 0.3]);
    }
}
