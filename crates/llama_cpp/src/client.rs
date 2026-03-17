//! HTTP client for the llama.cpp server.

use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use secrecy::{ExposeSecret, SecretString};
use tracing::{debug, trace};

use crate::error::{LlamaCppError, Result};

/// Default base URL for the llama.cpp server.
pub const DEFAULT_BASE_URL: &str = "http://localhost:8080";

/// API endpoints.
pub mod endpoints {
    /// Chat completions endpoint.
    pub const CHAT_COMPLETIONS: &str = "/chat/completion";
    /// Text completions endpoint.
    pub const COMPLETIONS: &str = "/completion";
    /// Tokenize endpoint.
    pub const TOKENIZE: &str = "/tokenize";
    /// Embeddings endpoint.
    pub const EMBEDDING: &str = "/embedding";
    /// Health endpoint.
    pub const HEALTH: &str = "/health";
    /// Slots endpoint.
    pub const SLOTS: &str = "/slots";
}

/// Default retry configuration.
pub mod retry {
    /// Maximum number of retry attempts.
    pub const MAX_RETRIES: u32 = 3;
    /// Initial retry delay in milliseconds.
    pub const INITIAL_DELAY_MS: u64 = 500;
    /// Maximum retry delay in milliseconds.
    pub const MAX_DELAY_MS: u64 = 10000;
    /// Exponential backoff multiplier.
    pub const BACKOFF_MULTIPLIER: f64 = 2.0;
}

/// A client for the llama.cpp server.
#[derive(Debug, Clone)]
pub struct LlamaCppClient {
    inner: Arc<ClientInner>,
}

#[derive(Debug)]
#[allow(dead_code)]
struct ClientInner {
    http: reqwest::Client,
    base_url: String,
    max_retries: u32,
    retry_delay: Duration,
    api_key: Option<SecretString>,
}

impl LlamaCppClient {
    /// Create a new llama.cpp client with default configuration.
    pub fn new() -> Result<Self> {
        Self::with_config(ClientConfig::default())
    }

    /// Create a new llama.cpp client with a custom base URL.
    pub fn with_base_url(url: impl Into<String>) -> Result<Self> {
        Self::with_config(ClientConfig::default().base_url(url))
    }

    /// Create a client with a custom configuration.
    pub fn with_config(config: ClientConfig) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        // Add API key if provided
        if let Some(ref api_key) = config.api_key {
            headers.insert(
                reqwest::header::AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {}", api_key.expose_secret()))
                    .map_err(|_| LlamaCppError::Config {
                        message: "Invalid API key".to_string(),
                    })?,
            );
        }

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .connect_timeout(Duration::from_secs(10))
            .timeout(config.timeout)
            .build()
            .map_err(|e| LlamaCppError::Config {
                message: format!("Failed to create HTTP client: {e}"),
            })?;

        Ok(Self {
            inner: Arc::new(ClientInner {
                http,
                base_url: config.base_url,
                max_retries: config.max_retries,
                retry_delay: config.retry_delay,
                api_key: config.api_key,
            }),
        })
    }

    /// Get the chat completions API client.
    #[cfg(feature = "chat")]
    pub fn chat(&self) -> crate::chat::Chat<'_> {
        crate::chat::Chat::new(self)
    }

    /// Get the completions API client.
    #[cfg(feature = "completion")]
    pub fn completion(&self) -> crate::completion::Completion<'_> {
        crate::completion::Completion::new(self)
    }

    /// Get the tokenize API client.
    #[cfg(feature = "tokenize")]
    pub fn tokenize(&self) -> crate::tokenize::Tokenize<'_> {
        crate::tokenize::Tokenize::new(self)
    }

    /// Get the embeddings API client.
    #[cfg(feature = "embeddings")]
    pub fn embeddings(&self) -> crate::embeddings::Embeddings<'_> {
        crate::embeddings::Embeddings::new(self)
    }

    /// Get the health API client.
    #[cfg(feature = "health")]
    pub fn health(&self) -> crate::health::Health<'_> {
        crate::health::Health::new(self)
    }

    /// Get the slots API client.
    #[cfg(feature = "slots")]
    pub fn slots(&self) -> crate::slots::Slots<'_> {
        crate::slots::Slots::new(self)
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

        Err(LlamaCppError::RetryExhausted {
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
            .map_err(LlamaCppError::from)?;

        Ok(response)
    }

    /// Make a POST request to the API.
    pub(crate) async fn post(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> Result<reqwest::Response> {
        let url = format!("{}{}", self.inner.base_url, path);
        trace!(url = %url, body = %body, "Making POST request");

        let response = self
            .inner
            .http
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(LlamaCppError::from)?;

        Ok(response)
    }

    /// Parse a response or return an error.
    pub(crate) async fn handle_response(&self, response: reqwest::Response) -> Result<serde_json::Value> {
        let status = response.status();

        if status.is_success() {
            let body = response.json::<serde_json::Value>().await.map_err(|e| {
                LlamaCppError::Internal {
                    message: format!("Failed to parse JSON response: {e}"),
                }
            })?;
            Ok(body)
        } else if status == reqwest::StatusCode::NOT_FOUND {
            let body = response.text().await.unwrap_or_default();
            Err(LlamaCppError::NotFound {
                resource: "resource".to_string(),
                id: body,
            })
        } else {
            Err(LlamaCppError::from_response(response).await)
        }
    }

    /// Get a reference to the HTTP client.
    pub(crate) fn http(&self) -> &reqwest::Client {
        &self.inner.http
    }
}

impl Default for LlamaCppClient {
    fn default() -> Self {
        Self::new().expect("Failed to create default client")
    }
}

/// Configuration for the llama.cpp client.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub(crate) base_url: String,
    pub(crate) timeout: Duration,
    pub(crate) max_retries: u32,
    pub(crate) retry_delay: Duration,
    pub(crate) api_key: Option<SecretString>,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
            timeout: Duration::from_secs(300),
            max_retries: retry::MAX_RETRIES,
            retry_delay: Duration::from_millis(retry::INITIAL_DELAY_MS),
            api_key: None,
        }
    }
}

impl ClientConfig {
    /// Create a new configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a custom base URL.
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
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

    /// Set the API key for authentication.
    pub fn api_key(mut self, key: impl Into<SecretString>) -> Self {
        self.api_key = Some(key.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_config_default() {
        let config = ClientConfig::default();
        assert_eq!(config.base_url, DEFAULT_BASE_URL);
        assert_eq!(config.max_retries, retry::MAX_RETRIES);
    }

    #[test]
    fn test_client_config_builder() {
        let config = ClientConfig::new()
            .base_url("http://custom:8080")
            .max_retries(5);

        assert_eq!(config.base_url, "http://custom:8080");
        assert_eq!(config.max_retries, 5);
    }

    #[test]
    fn test_client_creation() {
        let client = LlamaCppClient::new().unwrap();
        assert_eq!(client.base_url(), DEFAULT_BASE_URL);
    }

    #[test]
    fn test_client_with_base_url() {
        let client = LlamaCppClient::with_base_url("http://localhost:8081").unwrap();
        assert_eq!(client.base_url(), "http://localhost:8081");
    }
}
