//! HTTP client for the AI21 API.

use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use secrecy::{ExposeSecret, SecretString};
use tracing::{debug, trace};

use crate::constants::{headers, retry, DEFAULT_API_VERSION, DEFAULT_BASE_URL};
use crate::error::{Ai21Error, Result};

#[cfg(feature = "chat")]
use crate::chat::ChatEndpoint;
#[cfg(feature = "completions")]
use crate::completions::CompletionsEndpoint;
#[cfg(feature = "rag")]
use crate::rag::RagEndpoint;
#[cfg(feature = "tokenize")]
use crate::tokenize::TokenizeEndpoint;

/// A client for the AI21 API.
#[derive(Debug, Clone)]
pub struct Ai21Client {
    pub(crate) inner: Arc<ClientInner>,
}

#[derive(Debug)]
pub(crate) struct ClientInner {
    pub http: reqwest::Client,
    // Stored for potential future use (e.g., custom auth schemes)
    #[allow(dead_code)]
    pub api_key: SecretString,
    pub base_url: String,
    pub api_version: String,
    pub max_retries: u32,
    pub retry_delay: Duration,
}

impl Ai21Client {
    /// Create a new AI21 client with the given API key.
    pub fn new(api_key: impl Into<SecretString>) -> Result<Self> {
        Self::with_config(ClientConfig::new(api_key))
    }

    /// Create a client with a custom configuration.
    pub fn with_config(config: ClientConfig) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let auth_header = format!("Bearer {}", config.api_key.expose_secret());
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&auth_header).map_err(|_| Ai21Error::Config {
                message: "Invalid API key".to_string(),
            })?,
        );

        headers.insert(
            headers::CLIENT_VERSION,
            HeaderValue::from_static(crate::VERSION),
        );

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(config.timeout)
            .build()
            .map_err(|e| Ai21Error::Config {
                message: format!("Failed to create HTTP client: {e}"),
            })?;

        Ok(Self {
            inner: Arc::new(ClientInner {
                http,
                api_key: config.api_key,
                base_url: config.base_url,
                api_version: config.api_version,
                max_retries: config.max_retries,
                retry_delay: config.retry_delay,
            }),
        })
    }

    /// Get the Chat API client.
    #[cfg(feature = "chat")]
    pub fn chat(&self) -> ChatEndpoint<'_> {
        ChatEndpoint { client: self }
    }

    /// Get the Completions API client.
    #[cfg(feature = "completions")]
    pub fn completions(&self) -> CompletionsEndpoint<'_> {
        CompletionsEndpoint { client: self }
    }

    /// Get the RAG API client.
    #[cfg(feature = "rag")]
    pub fn rag(&self) -> RagEndpoint<'_> {
        RagEndpoint { client: self }
    }

    /// Get the Tokenize API client.
    #[cfg(feature = "tokenize")]
    pub fn tokenize(&self) -> TokenizeEndpoint<'_> {
        TokenizeEndpoint { client: self }
    }

    /// Get the base URL.
    pub fn base_url(&self) -> &str {
        &self.inner.base_url
    }

    /// Get the API version.
    pub fn api_version(&self) -> &str {
        &self.inner.api_version
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

                    // Check for retry-after header
                    if let Some(retry_after) = err.retry_after() {
                        delay = retry_after;
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

        Err(Ai21Error::RetryExhausted {
            attempts,
            last_error: Box::new(last_error.unwrap()),
        })
    }

    /// Make a POST request to the API.
    pub(crate) async fn post(&self, path: &str, body: serde_json::Value) -> Result<reqwest::Response> {
        let url = format!("{}{}", self.inner.base_url, path);

        trace!(url = %url, body = %body, "Making POST request");

        let response = self
            .inner
            .http
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(Ai21Error::from)?;

        Ok(response)
    }

    /// Parse a response or return an error.
    pub(crate) async fn handle_response(&self, response: reqwest::Response) -> Result<serde_json::Value> {
        let status = response.status();

        if status.is_success() {
            let body = response.json::<serde_json::Value>().await.map_err(|e| {
                Ai21Error::Internal {
                    message: format!("Failed to parse JSON response: {e}"),
                }
            })?;
            Ok(body)
        } else {
            Err(Ai21Error::from_response(response).await)
        }
    }
}

/// Configuration for the AI21 client.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub(crate) api_key: SecretString,
    pub(crate) base_url: String,
    pub(crate) api_version: String,
    pub(crate) timeout: Duration,
    pub(crate) max_retries: u32,
    pub(crate) retry_delay: Duration,
}

impl ClientConfig {
    /// Create a new configuration with the given API key.
    pub fn new(api_key: impl Into<SecretString>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: DEFAULT_BASE_URL.to_string(),
            api_version: DEFAULT_API_VERSION.to_string(),
            timeout: Duration::from_secs(120),
            max_retries: retry::MAX_RETRIES,
            retry_delay: Duration::from_millis(retry::INITIAL_DELAY_MS),
        }
    }

    /// Set a custom base URL.
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Set a custom API version.
    pub fn api_version(mut self, version: impl Into<String>) -> Self {
        self.api_version = version.into();
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

    /// Get the base URL.
    pub fn get_base_url(&self) -> &str {
        &self.base_url
    }

    /// Get the API version.
    pub fn get_api_version(&self) -> &str {
        &self.api_version
    }

    /// Get the timeout.
    pub fn get_timeout(&self) -> Duration {
        self.timeout
    }

    /// Get the maximum number of retries.
    pub fn get_max_retries(&self) -> u32 {
        self.max_retries
    }

    /// Get the retry delay.
    pub fn get_retry_delay(&self) -> Duration {
        self.retry_delay
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_config_builder() {
        let config = ClientConfig::new("test-key")
            .base_url("https://custom.api.com")
            .api_version("v2")
            .max_retries(5);

        assert_eq!(config.get_base_url(), "https://custom.api.com");
        assert_eq!(config.get_api_version(), "v2");
        assert_eq!(config.get_max_retries(), 5);
    }

    #[test]
    fn test_client_creation() {
        let client = Ai21Client::new("test-key").unwrap();
        assert_eq!(client.base_url(), DEFAULT_BASE_URL);
    }
}
