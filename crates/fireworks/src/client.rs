//! HTTP client for the Fireworks AI API.

use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use secrecy::{ExposeSecret, SecretString};
use tracing::{debug, trace};

use crate::error::{FireworksError, Result};

/// Default base URL for the Fireworks AI API.
pub const DEFAULT_BASE_URL: &str = "https://api.fireworks.ai/inference/v1";

/// Default app name.
pub const DEFAULT_APP_NAME: &str = "fireworks-ai-sdk";

/// API endpoints.
#[allow(dead_code)]
pub mod endpoints {
    /// Chat completions endpoint.
    pub const CHAT_COMPLETIONS: &str = "/chat/completions";
    /// Completions endpoint.
    pub const COMPLETIONS: &str = "/completions";
    /// Embeddings endpoint.
    pub const EMBEDDINGS: &str = "/embeddings";
    /// Fine-tuning endpoint.
    pub const FINE_TUNING: &str = "/fine-tunes";
    /// Models endpoint.
    pub const MODELS: &str = "/models";
    /// Files endpoint (for fine-tuning uploads).
    pub const FILES: &str = "/files";
    /// Image generation endpoint.
    pub const IMAGE_GENERATION: &str = "/image-generation";
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

/// A client for the Fireworks AI API.
#[derive(Debug, Clone)]
pub struct FireworksClient {
    inner: Arc<ClientInner>,
}

#[derive(Debug)]
struct ClientInner {
    http: reqwest::Client,
    api_key: SecretString,
    base_url: String,
    #[allow(dead_code)]
    app_name: String,
    max_retries: u32,
    retry_delay: Duration,
}

impl FireworksClient {
    /// Create a new Fireworks client with the given API key.
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
                .map_err(|_| FireworksError::Config {
                    message: "Invalid API key".to_string(),
                })?,
        );

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(config.timeout)
            .build()
            .map_err(|e| FireworksError::Config {
                message: format!("Failed to create HTTP client: {e}"),
            })?;

        Ok(Self {
            inner: Arc::new(ClientInner {
                http,
                api_key: config.api_key,
                base_url: config.base_url,
                app_name: config.app_name,
                max_retries: config.max_retries,
                retry_delay: config.retry_delay,
            }),
        })
    }

    /// Get the chat completions API client.
    #[cfg(feature = "chat")]
    pub fn chat(&self) -> crate::chat::Chat<'_> {
        crate::chat::Chat::new(self)
    }

    /// Get the completions API client.
    #[cfg(feature = "completions")]
    pub fn completions(&self) -> crate::completions::Completions<'_> {
        crate::completions::Completions::new(self)
    }

    /// Get the embeddings API client.
    #[cfg(feature = "embeddings")]
    pub fn embeddings(&self) -> crate::embeddings::Embeddings<'_> {
        crate::embeddings::Embeddings::new(self)
    }

    /// Get the fine-tuning API client.
    #[cfg(feature = "fine-tuning")]
    pub fn fine_tuning(&self) -> crate::fine_tuning::FineTuning<'_> {
        crate::fine_tuning::FineTuning::new(self)
    }

    /// Get the image generation API client.
    #[cfg(feature = "image-generation")]
    pub fn images(&self) -> crate::image_generation::Images<'_> {
        crate::image_generation::Images::new(self)
    }

    /// Get the models API client.
    #[cfg(feature = "models")]
    pub fn models(&self) -> crate::models::Models<'_> {
        crate::models::Models::new(self)
    }

    /// Get the base URL.
    pub fn base_url(&self) -> &str {
        &self.inner.base_url
    }

    /// Get the API key (masked for security).
    pub fn api_key_masked(&self) -> String {
        let key = self.inner.api_key.expose_secret();
        if key.len() <= 8 {
            "***".to_string()
        } else {
            format!("{}...{}", &key[..4], &key[key.len() - 4..])
        }
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

        Err(FireworksError::RetryExhausted {
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
            .map_err(FireworksError::from)?;

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
            .map_err(FireworksError::from)?;

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
            .map_err(FireworksError::from)?;

        Ok(response)
    }

    /// Make a multipart POST request (for file uploads).
    pub(crate) async fn post_multipart(
        &self,
        path: &str,
        form: reqwest::multipart::Form,
    ) -> Result<reqwest::Response> {
        let url = format!("{}{}", self.inner.base_url, path);
        trace!(url = %url, "Making multipart POST request");

        let response = self
            .inner
            .http
            .post(&url)
            .header(AUTHORIZATION, format!("Bearer {}", self.inner.api_key.expose_secret()))
            .multipart(form)
            .send()
            .await
            .map_err(FireworksError::from)?;

        Ok(response)
    }

    /// Parse a response or return an error.
    pub(crate) async fn handle_response(&self, response: reqwest::Response) -> Result<serde_json::Value> {
        let status = response.status();

        if status.is_success() {
            let body = response.json::<serde_json::Value>().await.map_err(|e| {
                FireworksError::Internal {
                    message: format!("Failed to parse JSON response: {e}"),
                }
            })?;
            Ok(body)
        } else {
            Err(FireworksError::from_response(response).await)
        }
    }
}

/// Configuration for the Fireworks AI client.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub(crate) api_key: SecretString,
    pub(crate) base_url: String,
    pub(crate) app_name: String,
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
            app_name: DEFAULT_APP_NAME.to_string(),
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

    /// Set the app name.
    pub fn app_name(mut self, name: impl Into<String>) -> Self {
        self.app_name = name.into();
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
        let client = FireworksClient::new("test-key").unwrap();
        assert_eq!(client.base_url(), DEFAULT_BASE_URL);
    }

    #[test]
    fn test_api_key_masked() {
        let client = FireworksClient::new("my-secret-api-key").unwrap();
        let masked = client.api_key_masked();
        assert!(masked.starts_with("my-s"));
        assert!(masked.ends_with("-key"));
        assert!(masked.contains("..."));
    }
}
