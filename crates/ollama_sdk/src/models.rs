//! Models management API for Ollama.

use crate::client::OllamaClient;
use crate::client::endpoints;
use crate::error::{OllamaError, Result};
use crate::types::{
    CopyModelRequest, CreateModelRequest, CreateModelStatus, DeleteModelRequest,
    ListModelsResponse, ModelInfo, PullStatus, PushStatus, RunningModelsResponse, ShowModelRequest,
    ShowModelResponse, VersionResponse,
};

/// Client for the models API.
#[derive(Debug)]
pub struct Models<'a> {
    client: &'a OllamaClient,
}

impl<'a> Models<'a> {
    /// Create a new models client.
    pub fn new(client: &'a OllamaClient) -> Self {
        Self { client }
    }

    /// List all available models.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ollama_sdk::OllamaClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = OllamaClient::new("http://localhost:11434");
    ///
    /// let models = client.models().list().await?;
    /// for model in models {
    ///     println!("Model: {} (Size: {} MB)", model.name, model.size / 1_048_576);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list(&self) -> Result<Vec<ModelInfo>> {
        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                Box::pin(async move {
                    let response = client.get(endpoints::TAGS).await?;
                    let models_response: ListModelsResponse =
                        client.handle_response(response).await?;
                    Ok(models_response.models)
                })
            })
            .await
    }

    /// Show model information.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ollama_sdk::OllamaClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = OllamaClient::new("http://localhost:11434");
    ///
    /// let info = client.models().show("llama3.2").await?;
    /// println!("Modelfile: {:?}", info.modelfile);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn show(&self, model: impl AsRef<str>) -> Result<ShowModelResponse> {
        let request = ShowModelRequest {
            model: model.as_ref().to_string(),
        };

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let body = request.clone();
                Box::pin(async move {
                    let response = client.post(endpoints::SHOW, body).await?;
                    let info: ShowModelResponse = client.handle_response(response).await?;
                    Ok(info)
                })
            })
            .await
    }

    /// Pull a model from the registry.
    ///
    /// Returns a stream of progress updates.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ollama_sdk::OllamaClient;
    /// use futures::StreamExt;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = OllamaClient::new("http://localhost:11434");
    ///
    /// let mut stream = client.models().pull("llama3.2").await?;
    /// while let Some(status) = stream.next().await {
    ///     match status {
    ///         Ok(status) => {
    ///             if let Some(progress) = status.progress() {
    ///                 println!("Progress: {:.1}%", progress * 100.0);
    ///             }
    ///         }
    ///         Err(e) => eprintln!("Error: {}", e),
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "streaming")]
    pub async fn pull(&self, model: impl AsRef<str>) -> Result<PullStream> {
        use futures::StreamExt;

        let request = serde_json::json!({
            "model": model.as_ref(),
            "stream": true,
        });

        let url = format!("{}{}", self.client.base_url(), endpoints::PULL);

        tracing::debug!(model = %model.as_ref(), "Initiating model pull");

        let response = reqwest::Client::new()
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(OllamaError::from)?;

        let status = response.status();
        if !status.is_success() {
            return Err(OllamaError::from_response(response).await);
        }

        let stream = response
            .bytes_stream()
            .map(|bytes| match bytes {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    let lines: Vec<&str> = text.lines().collect();
                    let results: Vec<Result<PullStatus>> = lines
                        .into_iter()
                        .filter(|line| !line.is_empty())
                        .map(|line| match serde_json::from_str::<PullStatus>(line) {
                            Ok(status) => Ok(status),
                            Err(e) => Err(OllamaError::Stream {
                                message: format!("Failed to parse NDJSON: {e}"),
                            }),
                        })
                        .collect();

                    futures::stream::iter(results)
                }
                Err(e) => futures::stream::iter(vec![Err(OllamaError::Stream {
                    message: format!("Stream error: {e}"),
                })]),
            })
            .flatten();

        Ok(PullStream::new(stream))
    }

    /// Pull a model without streaming.
    ///
    /// This waits for the pull to complete before returning.
    pub async fn pull_blocking(&self, model: impl AsRef<str>) -> Result<()> {
        let request = serde_json::json!({
            "model": model.as_ref(),
            "stream": false,
        });

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let body = request.clone();
                Box::pin(async move {
                    let response = client.post(endpoints::PULL, body).await?;
                    client
                        .handle_response::<serde_json::Value>(response)
                        .await?;
                    Ok(())
                })
            })
            .await
    }

    /// Push a model to a registry.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ollama_sdk::OllamaClient;
    /// use futures::StreamExt;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = OllamaClient::new("http://localhost:11434");
    ///
    /// let mut stream = client.models().push("username/model").await?;
    /// while let Some(status) = stream.next().await {
    ///     match status {
    ///         Ok(status) => println!("Status: {}", status.status),
    ///         Err(e) => eprintln!("Error: {}", e),
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "streaming")]
    pub async fn push(&self, model: impl AsRef<str>) -> Result<PushStream> {
        use futures::StreamExt;

        let request = serde_json::json!({
            "model": model.as_ref(),
            "stream": true,
        });

        let url = format!("{}{}", self.client.base_url(), endpoints::PUSH);

        tracing::debug!(model = %model.as_ref(), "Initiating model push");

        let response = reqwest::Client::new()
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(OllamaError::from)?;

        let status = response.status();
        if !status.is_success() {
            return Err(OllamaError::from_response(response).await);
        }

        let stream = response
            .bytes_stream()
            .map(|bytes| match bytes {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    let lines: Vec<&str> = text.lines().collect();
                    let results: Vec<Result<PushStatus>> = lines
                        .into_iter()
                        .filter(|line| !line.is_empty())
                        .map(|line| match serde_json::from_str::<PushStatus>(line) {
                            Ok(status) => Ok(status),
                            Err(e) => Err(OllamaError::Stream {
                                message: format!("Failed to parse NDJSON: {e}"),
                            }),
                        })
                        .collect();

                    futures::stream::iter(results)
                }
                Err(e) => futures::stream::iter(vec![Err(OllamaError::Stream {
                    message: format!("Stream error: {e}"),
                })]),
            })
            .flatten();

        Ok(PushStream::new(stream))
    }

    /// Create a new model from a Modelfile.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ollama_sdk::OllamaClient;
    /// use futures::StreamExt;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = OllamaClient::new("http://localhost:11434");
    ///
    /// let modelfile = r#"
    /// FROM llama3.2
    /// SYSTEM You are a helpful assistant.
    /// ""#;
    ///
    /// let mut stream = client.models().create("my-model", modelfile).await?;
    /// while let Some(status) = stream.next().await {
    ///     match status {
    ///         Ok(status) => println!("Status: {}", status.status),
    ///         Err(e) => eprintln!("Error: {}", e),
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "streaming")]
    pub async fn create(
        &self,
        model: impl AsRef<str>,
        modelfile: impl AsRef<str>,
    ) -> Result<CreateModelStream> {
        use futures::StreamExt;

        let request = CreateModelRequest {
            model: model.as_ref().to_string(),
            modelfile: modelfile.as_ref().to_string(),
            path: None,
            quantize: None,
            stream: Some(true),
        };

        let url = format!("{}{}", self.client.base_url(), endpoints::CREATE);

        tracing::debug!(model = %model.as_ref(), "Creating model");

        let response = reqwest::Client::new()
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(OllamaError::from)?;

        let status = response.status();
        if !status.is_success() {
            return Err(OllamaError::from_response(response).await);
        }

        let stream = response
            .bytes_stream()
            .map(|bytes| match bytes {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    let lines: Vec<&str> = text.lines().collect();
                    let results: Vec<Result<CreateModelStatus>> = lines
                        .into_iter()
                        .filter(|line| !line.is_empty())
                        .map(
                            |line| match serde_json::from_str::<CreateModelStatus>(line) {
                                Ok(status) => Ok(status),
                                Err(e) => Err(OllamaError::Stream {
                                    message: format!("Failed to parse NDJSON: {e}"),
                                }),
                            },
                        )
                        .collect();

                    futures::stream::iter(results)
                }
                Err(e) => futures::stream::iter(vec![Err(OllamaError::Stream {
                    message: format!("Stream error: {e}"),
                })]),
            })
            .flatten();

        Ok(CreateModelStream::new(stream))
    }

    /// Copy a model.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ollama_sdk::OllamaClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = OllamaClient::new("http://localhost:11434");
    ///
    /// client.models().copy("llama3.2", "my-llama3.2").await?;
    /// println!("Model copied successfully");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn copy(&self, source: impl AsRef<str>, destination: impl AsRef<str>) -> Result<()> {
        let request = CopyModelRequest {
            source: source.as_ref().to_string(),
            destination: destination.as_ref().to_string(),
        };

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let body = request.clone();
                Box::pin(async move {
                    let response = client.post(endpoints::COPY, body).await?;
                    // COPY returns empty body on success
                    if response.status().is_success() {
                        Ok(())
                    } else {
                        Err(OllamaError::from_response(response).await)
                    }
                })
            })
            .await
    }

    /// Delete a model.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ollama_sdk::OllamaClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = OllamaClient::new("http://localhost:11434");
    ///
    /// client.models().delete("old-model").await?;
    /// println!("Model deleted successfully");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn delete(&self, model: impl AsRef<str>) -> Result<()> {
        let request = DeleteModelRequest {
            model: model.as_ref().to_string(),
        };

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let body = request.clone();
                Box::pin(async move {
                    let _response = client.delete(endpoints::DELETE).await?;
                    let body_json = serde_json::to_value(&body)?;
                    // DELETE uses request body, so we need to handle it specially
                    let response = client.post(endpoints::DELETE, body_json).await?;
                    if response.status().is_success() {
                        Ok(())
                    } else {
                        Err(OllamaError::from_response(response).await)
                    }
                })
            })
            .await
    }

    /// List running models.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ollama_sdk::OllamaClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = OllamaClient::new("http://localhost:11434");
    ///
    /// let running = client.models().running().await?;
    /// for model in running {
    ///     println!("Running: {} (VRAM: {} MB)", model.name, model.size_vram / 1_048_576);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn running(&self) -> Result<Vec<crate::types::RunningModel>> {
        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                Box::pin(async move {
                    let response = client.get(endpoints::PS).await?;
                    let models_response: RunningModelsResponse =
                        client.handle_response(response).await?;
                    Ok(models_response.models)
                })
            })
            .await
    }

    /// Get the Ollama version.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ollama_sdk::OllamaClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = OllamaClient::new("http://localhost:11434");
    ///
    /// let version = client.models().version().await?;
    /// println!("Ollama version: {}", version);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn version(&self) -> Result<String> {
        let response = self.client.get(endpoints::VERSION).await?;
        let version_response: VersionResponse = self.client.handle_response(response).await?;
        Ok(version_response.version)
    }
}

/// A pull status stream.
#[cfg(feature = "streaming")]
pub struct PullStream {
    inner: std::pin::Pin<Box<dyn futures::Stream<Item = Result<PullStatus>> + Send>>,
}

#[cfg(feature = "streaming")]
impl PullStream {
    fn new<S>(stream: S) -> Self
    where
        S: futures::Stream<Item = Result<PullStatus>> + Send + 'static,
    {
        Self {
            inner: Box::pin(stream),
        }
    }
}

#[cfg(feature = "streaming")]
impl futures::Stream for PullStream {
    type Item = Result<PullStatus>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}

#[cfg(feature = "streaming")]
impl std::fmt::Debug for PullStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PullStream").finish()
    }
}

/// A push status stream.
#[cfg(feature = "streaming")]
pub struct PushStream {
    inner: std::pin::Pin<Box<dyn futures::Stream<Item = Result<PushStatus>> + Send>>,
}

#[cfg(feature = "streaming")]
impl PushStream {
    fn new<S>(stream: S) -> Self
    where
        S: futures::Stream<Item = Result<PushStatus>> + Send + 'static,
    {
        Self {
            inner: Box::pin(stream),
        }
    }
}

#[cfg(feature = "streaming")]
impl futures::Stream for PushStream {
    type Item = Result<PushStatus>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}

#[cfg(feature = "streaming")]
impl std::fmt::Debug for PushStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PushStream").finish()
    }
}

/// A create model status stream.
#[cfg(feature = "streaming")]
pub struct CreateModelStream {
    inner: std::pin::Pin<Box<dyn futures::Stream<Item = Result<CreateModelStatus>> + Send>>,
}

#[cfg(feature = "streaming")]
impl CreateModelStream {
    fn new<S>(stream: S) -> Self
    where
        S: futures::Stream<Item = Result<CreateModelStatus>> + Send + 'static,
    {
        Self {
            inner: Box::pin(stream),
        }
    }
}

#[cfg(feature = "streaming")]
impl futures::Stream for CreateModelStream {
    type Item = Result<CreateModelStatus>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}

#[cfg(feature = "streaming")]
impl std::fmt::Debug for CreateModelStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CreateModelStream").finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // These tests verify the code compiles
    // Actual integration tests require a running Ollama instance

    #[test]
    fn test_models_client_creation() {
        // Just verifies the code compiles
    }

    #[test]
    fn test_pull_status_progress() {
        let status = PullStatus {
            status: "pulling".to_string(),
            digest: Some("sha256:abc123".to_string()),
            total: Some(1000),
            completed: Some(500),
        };

        assert_eq!(status.progress(), Some(0.5));
        assert!(!status.is_complete());

        let complete = PullStatus {
            status: "success".to_string(),
            digest: None,
            total: None,
            completed: None,
        };
        assert!(complete.is_complete());
    }
}
