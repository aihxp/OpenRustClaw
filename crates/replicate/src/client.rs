//! HTTP client for the Replicate API.

use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use secrecy::{ExposeSecret, SecretString};
use tracing::{debug, trace};

use crate::error::{ReplicateError, Result};

/// Default base URL for the Replicate API.
pub const DEFAULT_BASE_URL: &str = "https://api.replicate.com/v1";

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

/// A client for the Replicate API.
#[derive(Debug, Clone)]
pub struct ReplicateClient {
    inner: Arc<ClientInner>,
}

#[derive(Debug)]
struct ClientInner {
    http: reqwest::Client,
    api_token: SecretString,
    base_url: String,
    max_retries: u32,
    retry_delay: Duration,
}

impl ReplicateClient {
    /// Create a new Replicate client with the given API token.
    pub fn new(api_token: impl Into<SecretString>) -> Result<Self> {
        Self::with_config(ClientConfig::new(api_token))
    }

    /// Create a client with a custom configuration.
    pub fn with_config(config: ClientConfig) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", config.api_token.expose_secret()))
                .map_err(|_| ReplicateError::Config {
                    message: "Invalid API token".to_string(),
                })?,
        );

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .connect_timeout(Duration::from_secs(10))
            .timeout(config.timeout)
            .build()
            .map_err(|e| ReplicateError::Config {
                message: format!("Failed to create HTTP client: {e}"),
            })?;

        Ok(Self {
            inner: Arc::new(ClientInner {
                http,
                api_token: config.api_token,
                base_url: config.base_url,
                max_retries: config.max_retries,
                retry_delay: config.retry_delay,
            }),
        })
    }

    /// Get the predictions API client.
    pub fn predictions(&self) -> crate::predictions::Predictions<'_> {
        crate::predictions::Predictions::new(self)
    }

    /// Get the models API client.
    #[cfg(feature = "models")]
    pub fn models(&self) -> crate::models::Models<'_> {
        crate::models::Models::new(self)
    }

    /// Get the deployments API client.
    #[cfg(feature = "deployments")]
    pub fn deployments(&self) -> crate::deployments::Deployments<'_> {
        crate::deployments::Deployments::new(self)
    }

    /// Get the trainings API client.
    #[cfg(feature = "trainings")]
    pub fn trainings(&self) -> crate::trainings::Trainings<'_> {
        crate::trainings::Trainings::new(self)
    }

    /// Get the streaming API client.
    #[cfg(feature = "streaming")]
    pub fn streaming(&self) -> crate::streaming::Streaming<'_> {
        crate::streaming::Streaming::new(self)
    }

    /// Get the base URL.
    pub fn base_url(&self) -> &str {
        &self.inner.base_url
    }

    /// Get the API token.
    pub fn api_token(&self) -> &str {
        self.inner.api_token.expose_secret()
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

        Err(ReplicateError::RetryExhausted {
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
            .map_err(ReplicateError::from)?;

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
            .map_err(ReplicateError::from)?;

        Ok(response)
    }

    /// Make a POST request with custom headers.
    pub(crate) async fn post_with_headers(
        &self,
        path: &str,
        body: serde_json::Value,
        headers: HeaderMap,
    ) -> Result<reqwest::Response> {
        let url = format!("{}{}", self.inner.base_url, path);
        trace!(url = %url, body = %body, "Making POST request with headers");

        let response = self
            .inner
            .http
            .post(&url)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .map_err(ReplicateError::from)?;

        Ok(response)
    }

    /// Make a PATCH request to the API.
    pub(crate) async fn patch(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> Result<reqwest::Response> {
        let url = format!("{}{}", self.inner.base_url, path);
        trace!(url = %url, body = %body, "Making PATCH request");

        let response = self
            .inner
            .http
            .patch(&url)
            .json(&body)
            .send()
            .await
            .map_err(ReplicateError::from)?;

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
            .map_err(ReplicateError::from)?;

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
                .map_err(|e| ReplicateError::Internal {
                    message: format!("Failed to parse JSON response: {e}"),
                })?;
            Ok(body)
        } else if status == reqwest::StatusCode::NOT_FOUND {
            let body = response.text().await.unwrap_or_default();
            Err(ReplicateError::NotFound {
                resource: "resource".to_string(),
                id: body,
            })
        } else {
            Err(ReplicateError::from_response(response).await)
        }
    }

    /// Parse an empty response (for DELETE operations).
    pub(crate) async fn handle_empty_response(&self, response: reqwest::Response) -> Result<()> {
        let status = response.status();

        if status.is_success() {
            Ok(())
        } else {
            Err(ReplicateError::from_response(response).await)
        }
    }
}

/// Configuration for the Replicate client.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub(crate) api_token: SecretString,
    pub(crate) base_url: String,
    pub(crate) timeout: Duration,
    pub(crate) max_retries: u32,
    pub(crate) retry_delay: Duration,
}

impl ClientConfig {
    /// Create a new configuration with the given API token.
    pub fn new(api_token: impl Into<SecretString>) -> Self {
        Self {
            api_token: api_token.into(),
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
        let config = ClientConfig::new("test-token")
            .base_url("https://custom.api.com")
            .max_retries(5);

        assert_eq!(config.base_url, "https://custom.api.com");
        assert_eq!(config.max_retries, 5);
    }

    #[test]
    fn test_client_creation() {
        let client = ReplicateClient::new("test-token").unwrap();
        assert_eq!(client.base_url(), DEFAULT_BASE_URL);
        assert_eq!(client.api_token(), "test-token");
    }
}
