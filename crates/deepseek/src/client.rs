//! HTTP client for the DeepSeek API.

use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use secrecy::{ExposeSecret, SecretString};
use tracing::{debug, trace};

use crate::error::{DeepSeekError, Result};

/// Default base URL for the DeepSeek API.
pub const DEFAULT_BASE_URL: &str = "https://api.deepseek.com";

/// API endpoints.
pub mod endpoints {
    /// Chat completions endpoint.
    pub const CHAT_COMPLETIONS: &str = "/chat/completions";
    /// Models endpoint.
    pub const MODELS: &str = "/models";
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

/// A client for the DeepSeek API.
#[derive(Debug, Clone)]
pub struct DeepSeekClient {
    inner: Arc<ClientInner>,
}

#[derive(Debug)]
struct ClientInner {
    http: reqwest::Client,
    api_key: SecretString,
    base_url: String,
    max_retries: u32,
    retry_delay: Duration,
}

impl DeepSeekClient {
    /// Create a new DeepSeek client with the given API key.
    pub fn new(api_key: impl Into<SecretString>) -> Result<Self> {
        Self::with_config(ClientConfig::new(api_key))
    }

    /// Create a client with a custom configuration.
    pub fn with_config(config: ClientConfig) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", config.api_key.expose_secret()))
                .map_err(|_| DeepSeekError::Config {
                    message: "Invalid API key".to_string(),
                })?,
        );

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(config.timeout)
            .build()
            .map_err(|e| DeepSeekError::Config {
                message: format!("Failed to create HTTP client: {e}"),
            })?;

        Ok(Self {
            inner: Arc::new(ClientInner {
                http,
                api_key: config.api_key,
                base_url: config.base_url,
                max_retries: config.max_retries,
                retry_delay: config.retry_delay,
            }),
        })
    }

    /// Get the chat completions API client.
    pub fn chat(&self) -> crate::chat::Chat<'_> {
        crate::chat::Chat::new(self)
    }

    /// Get the models API client.
    pub fn models(&self) -> crate::models::Models<'_> {
        crate::models::Models::new(self)
    }

    /// Get the base URL.
    pub fn base_url(&self) -> &str {
        &self.inner.base_url
    }

    /// Get the API key.
    pub fn api_key(&self) -> &str {
        self.inner.api_key.expose_secret()
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

        Err(DeepSeekError::RetryExhausted {
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
            .map_err(DeepSeekError::from)?;

        Ok(response)
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
            .map_err(DeepSeekError::from)?;

        Ok(response)
    }

    /// Make a DELETE request to the API.
    #[allow(dead_code)]
    pub(crate) async fn delete(&self, path: &str) -> Result<reqwest::Response> {
        let url = format!("{}{}", self.inner.base_url, path);
        trace!(url = %url, "Making DELETE request");

        let response = self
            .inner
            .http
            .delete(&url)
            .send()
            .await
            .map_err(DeepSeekError::from)?;

        Ok(response)
    }

    /// Parse a response or return an error.
    pub(crate) async fn handle_response(&self, response: reqwest::Response) -> Result<serde_json::Value> {
        let status = response.status();

        if status.is_success() {
            let body = response.json::<serde_json::Value>().await.map_err(|e| {
                DeepSeekError::Internal {
                    message: format!("Failed to parse JSON response: {e}"),
                }
            })?;
            Ok(body)
        } else if status == reqwest::StatusCode::NOT_FOUND {
            let body = response.text().await.unwrap_or_default();
            Err(DeepSeekError::NotFound {
                resource: "resource".to_string(),
                id: body,
            })
        } else {
            Err(DeepSeekError::from_response(response).await)
        }
    }
}

/// Configuration for the DeepSeek client.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub(crate) api_key: SecretString,
    pub(crate) base_url: String,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_config_builder() {
        let config = ClientConfig::new("test-key")
            .base_url("https://custom.api.com")
            .max_retries(5);

        assert_eq!(config.base_url, "https://custom.api.com");
        assert_eq!(config.max_retries, 5);
    }

    #[test]
    fn test_client_creation() {
        let client = DeepSeekClient::new("test-key").unwrap();
        assert_eq!(client.base_url(), DEFAULT_BASE_URL);
        assert_eq!(client.api_key(), "test-key");
    }

    #[test]
    fn test_default_base_url() {
        assert_eq!(DEFAULT_BASE_URL, "https://api.deepseek.com");
    }
}
