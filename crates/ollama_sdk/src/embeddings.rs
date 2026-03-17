//! Embeddings API for Ollama.

use crate::client::OllamaClient;
use crate::client::endpoints;
use crate::error::Result;
use crate::types::{EmbeddingResponse, KeepAlive, Options};

/// Client for the embeddings API.
#[derive(Debug)]
pub struct Embeddings<'a> {
    client: &'a OllamaClient,
}

impl<'a> Embeddings<'a> {
    /// Create a new embeddings client.
    pub fn new(client: &'a OllamaClient) -> Self {
        Self { client }
    }

    /// Generate embeddings for the given prompt.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ollama_sdk::OllamaClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = OllamaClient::new("http://localhost:11434");
    ///
    /// let response = client.embeddings()
    ///     .generate("nomic-embed-text", "Hello, world!")
    ///     .await?;
    ///
    /// println!("Embedding dimensions: {}", response.embedding.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn generate(
        &self,
        model: impl AsRef<str>,
        prompt: impl AsRef<str>,
    ) -> Result<EmbeddingResponse> {
        let request = EmbeddingsRequest {
            model: model.as_ref().to_string(),
            prompt: prompt.as_ref().to_string(),
            options: None,
            keep_alive: None,
        };

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let body = request.clone();
                Box::pin(async move {
                    let response = client.post(endpoints::EMBEDDINGS, body).await?;
                    let embedding_response: EmbeddingResponse =
                        client.handle_response(response).await?;
                    Ok(embedding_response)
                })
            })
            .await
    }

    /// Generate embeddings with custom options.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ollama_sdk::{OllamaClient, Options};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = OllamaClient::new("http://localhost:11434");
    ///
    /// let options = Options::builder()
    ///     .num_thread(4)
    ///     .build();
    ///
    /// let response = client.embeddings()
    ///     .generate_with_options("nomic-embed-text", "Hello, world!", options)
    ///     .await?;
    ///
    /// println!("Embedding dimensions: {}", response.embedding.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn generate_with_options(
        &self,
        model: impl AsRef<str>,
        prompt: impl AsRef<str>,
        options: Options,
    ) -> Result<EmbeddingResponse> {
        let request = EmbeddingsRequest {
            model: model.as_ref().to_string(),
            prompt: prompt.as_ref().to_string(),
            options: Some(options),
            keep_alive: None,
        };

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let body = request.clone();
                Box::pin(async move {
                    let response = client.post(endpoints::EMBEDDINGS, body).await?;
                    let embedding_response: EmbeddingResponse =
                        client.handle_response(response).await?;
                    Ok(embedding_response)
                })
            })
            .await
    }

    /// Generate embeddings with keep_alive setting.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ollama_sdk::{OllamaClient, KeepAlive};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = OllamaClient::new("http://localhost:11434");
    ///
    /// let response = client.embeddings()
    ///     .generate_with_keep_alive("nomic-embed-text", "Hello, world!", KeepAlive::indefinitely())
    ///     .await?;
    ///
    /// println!("Embedding dimensions: {}", response.embedding.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn generate_with_keep_alive(
        &self,
        model: impl AsRef<str>,
        prompt: impl AsRef<str>,
        keep_alive: KeepAlive,
    ) -> Result<EmbeddingResponse> {
        let request = EmbeddingsRequest {
            model: model.as_ref().to_string(),
            prompt: prompt.as_ref().to_string(),
            options: None,
            keep_alive: Some(keep_alive),
        };

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let body = request.clone();
                Box::pin(async move {
                    let response = client.post(endpoints::EMBEDDINGS, body).await?;
                    let embedding_response: EmbeddingResponse =
                        client.handle_response(response).await?;
                    Ok(embedding_response)
                })
            })
            .await
    }

    /// Generate embeddings for multiple texts.
    ///
    /// This is a convenience method that batches multiple embedding requests.
    /// Note: Ollama's embeddings API processes one prompt at a time, so this
    /// method makes sequential requests.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ollama_sdk::OllamaClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = OllamaClient::new("http://localhost:11434");
    ///
    /// let texts = vec!["Hello", "World", "Ollama"];
    /// let embeddings = client.embeddings()
    ///     .generate_batch("nomic-embed-text", texts)
    ///     .await?;
    ///
    /// for (text, embedding) in embeddings {
    ///     println!("{}: {} dimensions", text, embedding.len());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn generate_batch(
        &self,
        model: impl AsRef<str>,
        prompts: Vec<impl AsRef<str>>,
    ) -> Result<Vec<(String, Vec<f32>)>> {
        let model = model.as_ref();
        let mut results = Vec::with_capacity(prompts.len());

        for prompt in prompts {
            let response = self.generate(model, &prompt).await?;
            results.push((prompt.as_ref().to_string(), response.embedding));
        }

        Ok(results)
    }
}

/// An embeddings request.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EmbeddingsRequest {
    /// The model name.
    pub model: String,
    /// The prompt to generate embeddings for.
    pub prompt: String,
    /// Additional model parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Options>,
    /// Controls how long the model stays loaded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_alive: Option<KeepAlive>,
}

/// Builder for embeddings requests.
#[derive(Debug, Clone)]
pub struct EmbeddingsRequestBuilder {
    model: String,
    prompt: String,
    options: Option<Options>,
    keep_alive: Option<KeepAlive>,
}

impl EmbeddingsRequestBuilder {
    /// Create a new builder.
    pub fn new(model: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            prompt: prompt.into(),
            options: None,
            keep_alive: None,
        }
    }

    /// Set options.
    pub fn options(mut self, options: Options) -> Self {
        self.options = Some(options);
        self
    }

    /// Set the keep_alive duration.
    pub fn keep_alive(mut self, keep_alive: KeepAlive) -> Self {
        self.keep_alive = Some(keep_alive);
        self
    }

    /// Set number of threads.
    pub fn num_thread(mut self, n: i32) -> Self {
        let opts = self.options.get_or_insert_with(Options::default);
        opts.num_thread = Some(n);
        self
    }

    /// Build the request.
    pub fn build(self) -> EmbeddingsRequest {
        EmbeddingsRequest {
            model: self.model,
            prompt: self.prompt,
            options: self.options,
            keep_alive: self.keep_alive,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embeddings_request_builder() {
        let request = EmbeddingsRequestBuilder::new("nomic-embed-text", "Hello")
            .num_thread(4)
            .build();

        assert_eq!(request.model, "nomic-embed-text");
        assert_eq!(request.prompt, "Hello");
        assert!(request.options.is_some());
        assert_eq!(request.options.unwrap().num_thread, Some(4));
    }

    #[test]
    fn test_embeddings_request_builder_keep_alive() {
        let request = EmbeddingsRequestBuilder::new("nomic-embed-text", "Hello")
            .keep_alive(KeepAlive::seconds(300))
            .build();

        assert!(matches!(request.keep_alive, Some(KeepAlive::Seconds(300))));
    }
}
