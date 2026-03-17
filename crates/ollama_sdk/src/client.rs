//! HTTP client for the Ollama API.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue};
use tracing::{debug, trace};

use crate::error::{OllamaError, Result};

/// Default base URL for the local Ollama instance.
pub const DEFAULT_BASE_URL: &str = "http://localhost:11434";

/// API endpoints.
pub(crate) mod endpoints {
    /// Chat completions endpoint.
    pub const CHAT: &str = "/api/chat";
    /// Generate completion endpoint.
    pub const GENERATE: &str = "/api/generate";
    /// List models endpoint.
    pub const TAGS: &str = "/api/tags";
    /// Show model info endpoint.
    pub const SHOW: &str = "/api/show";
    /// Create model endpoint.
    pub const CREATE: &str = "/api/create";
    /// Copy model endpoint.
    pub const COPY: &str = "/api/copy";
    /// Delete model endpoint.
    pub const DELETE: &str = "/api/delete";
    /// Pull model endpoint.
    pub const PULL: &str = "/api/pull";
    /// Push model endpoint.
    pub const PUSH: &str = "/api/push";
    /// Generate embeddings endpoint.
    pub const EMBEDDINGS: &str = "/api/embeddings";
    /// List running models endpoint.
    pub const PS: &str = "/api/ps";
    /// Version endpoint.
    pub const VERSION: &str = "/api/version";
}

/// Default retry configuration.
pub mod retry {
    /// Maximum number of retry attempts.
    pub const MAX_RETRIES: u32 = 3;
    /// Initial retry delay in milliseconds.
    pub const INITIAL_DELAY_MS: u64 = 1000;
    /// Maximum retry delay in milliseconds.
    pub const MAX_DELAY_MS: u64 = 32000;
    /// Exponential backoff multiplier.
    pub const BACKOFF_MULTIPLIER: f64 = 2.0;
}

/// A client for the Ollama API.
#[derive(Debug, Clone)]
pub struct OllamaClient {
    inner: Arc<ClientInner>,
}

#[derive(Debug)]
struct ClientInner {
    http: reqwest::Client,
    base_url: String,
    max_retries: u32,
    retry_delay: Duration,
}

impl OllamaClient {
    /// Create a new Ollama client with the default base URL.
    ///
    /// # Example
    ///
    /// ```
    /// use ollama_sdk::OllamaClient;
    ///
    /// let client = OllamaClient::new("http://localhost:11434");
    /// ```
    pub fn new(base_url: impl Into<String>) -> Self {
        Self::with_config(ClientConfig::new(base_url))
    }

    /// Create a client with a custom configuration.
    pub fn with_config(config: ClientConfig) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        // Add custom headers
        for (key, value) in config.custom_headers {
            headers.insert(key, value);
        }

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .connect_timeout(Duration::from_secs(10))
            .timeout(config.timeout)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            inner: Arc::new(ClientInner {
                http,
                base_url: config.base_url,
                max_retries: config.max_retries,
                retry_delay: config.retry_delay,
            }),
        }
    }

    /// Get the chat API client.
    pub fn chat(&self) -> crate::chat::Chat<'_> {
        crate::chat::Chat::new(self)
    }

    /// Get the generate API client.
    pub fn generate(&self) -> crate::generate::Generate<'_> {
        crate::generate::Generate::new(self)
    }

    /// Get the models API client.
    pub fn models(&self) -> crate::models::Models<'_> {
        crate::models::Models::new(self)
    }

    /// Get the embeddings API client.
    pub fn embeddings(&self) -> crate::embeddings::Embeddings<'_> {
        crate::embeddings::Embeddings::new(self)
    }

    /// Get the base URL.
    pub fn base_url(&self) -> &str {
        &self.inner.base_url
    }

    /// Execute a request with retry logic.
    pub(crate) async fn execute_with_retry<T, F>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send>>,
    {
        let mut attempts = 0;
        let mut delay = self.inner.retry_delay;
        let mut last_error = None;

        while attempts <= self.inner.max_retries {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(err) => {
                    if !err.is_retryable() || attempts == self.inner.max_retries {
                        return Err(err);
                    }

                    last_error = Some(err);
                    attempts += 1;

                    debug!(
                        attempt = attempts,
                        max_retries = self.inner.max_retries,
                        delay = ?delay,
                        "Retrying request"
                    );

                    tokio::time::sleep(delay).await;
                    delay = std::cmp::min(
                        delay.mul_f64(retry::BACKOFF_MULTIPLIER),
                        Duration::from_millis(retry::MAX_DELAY_MS),
                    );
                }
            }
        }

        Err(OllamaError::RetryExhausted {
            attempts,
            last_error: Box::new(last_error.unwrap()),
        })
    }

    /// Make a GET request to the API.
    pub(crate) async fn get(&self, path: &str) -> Result<reqwest::Response> {
        let url = format!("{}{}", self.inner.base_url, path);
        trace!(url = %url, "Making GET request");

        let response = self
            .inner
            .http
            .get(&url)
            .send()
            .await
            .map_err(OllamaError::from)?;

        Ok(response)
    }

    /// Make a POST request to the API.
    pub(crate) async fn post(&self, path: &str, body: impl Serialize) -> Result<reqwest::Response> {
        let url = format!("{}{}", self.inner.base_url, path);
        let json_body = serde_json::to_value(&body)?;
        trace!(url = %url, body = %json_body, "Making POST request");

        let response = self
            .inner
            .http
            .post(&url)
            .json(&json_body)
            .send()
            .await
            .map_err(OllamaError::from)?;

        Ok(response)
    }

    /// Make a DELETE request to the API.
    pub(crate) async fn delete(&self, path: &str) -> Result<reqwest::Response> {
        let url = format!("{}{}", self.inner.base_url, path);
        trace!(url = %url, "Making DELETE request");

        let response = self
            .inner
            .http
            .delete(&url)
            .send()
            .await
            .map_err(OllamaError::from)?;

        Ok(response)
    }

    /// Parse a response or return an error.
    pub(crate) async fn handle_response<T: serde::de::DeserializeOwned>(
        &self,
        response: reqwest::Response,
    ) -> Result<T> {
        let status = response.status();

        if status.is_success() {
            let body = response
                .json::<T>()
                .await
                .map_err(|e| OllamaError::Internal {
                    message: format!("Failed to parse JSON response: {e}"),
                })?;
            Ok(body)
        } else {
            Err(OllamaError::from_response(response).await)
        }
    }
}

/// Configuration for the Ollama client.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub(crate) base_url: String,
    pub(crate) timeout: Duration,
    pub(crate) max_retries: u32,
    pub(crate) retry_delay: Duration,
    pub(crate) custom_headers: HashMap<reqwest::header::HeaderName, HeaderValue>,
}

impl ClientConfig {
    /// Create a new configuration with the given base URL.
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            timeout: Duration::from_secs(120),
            max_retries: retry::MAX_RETRIES,
            retry_delay: Duration::from_millis(retry::INITIAL_DELAY_MS),
            custom_headers: HashMap::new(),
        }
    }

    /// Set the request timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the maximum number of retries.
    pub fn max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    /// Set the initial retry delay.
    pub fn retry_delay(mut self, delay: Duration) -> Self {
        self.retry_delay = delay;
        self
    }

    /// Add a custom header.
    pub fn header(
        mut self,
        name: impl AsRef<str>,
        value: impl AsRef<str>,
    ) -> crate::error::Result<Self> {
        let name =
            reqwest::header::HeaderName::from_bytes(name.as_ref().as_bytes()).map_err(|e| {
                OllamaError::Config {
                    message: format!("Invalid header name: {e}"),
                }
            })?;
        let value = HeaderValue::from_str(value.as_ref()).map_err(|e| OllamaError::Config {
            message: format!("Invalid header value: {e}"),
        })?;
        self.custom_headers.insert(name, value);
        Ok(self)
    }
}

// Import Serialize for the post method
use serde::Serialize;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_config_builder() {
        let config = ClientConfig::new("http://ollama.local:11434")
            .timeout(Duration::from_secs(60))
            .max_retries(5);

        assert_eq!(config.base_url, "http://ollama.local:11434");
        assert_eq!(config.max_retries, 5);
    }

    #[test]
    fn test_client_creation() {
        let client = OllamaClient::new("http://localhost:11434");
        assert_eq!(client.base_url(), "http://localhost:11434");
    }

    #[test]
    fn test_default_base_url() {
        assert_eq!(DEFAULT_BASE_URL, "http://localhost:11434");
    }

    #[test]
    fn test_custom_header() {
        use reqwest::header::HeaderName;

        let config = ClientConfig::new("http://localhost:11434")
            .header("X-Custom", "value")
            .unwrap();
        // Header names are normalized - check the value directly
        let header_name = HeaderName::from_bytes(b"x-custom").unwrap();
        assert!(config.custom_headers.contains_key(&header_name));
        assert_eq!(
            config
                .custom_headers
                .get(&header_name)
                .unwrap()
                .to_str()
                .unwrap(),
            "value"
        );
    }
}
