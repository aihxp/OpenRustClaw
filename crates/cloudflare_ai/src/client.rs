//! HTTP client for the Cloudflare Workers AI API.

use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use secrecy::{ExposeSecret, SecretString};
use tracing::{debug, trace};

use crate::error::{CloudflareAiError, Result};

/// Default base URL for the Cloudflare Workers AI API.
pub const DEFAULT_BASE_URL: &str = "https://api.cloudflare.com/client/v4/accounts";

/// API endpoints.
pub mod endpoints {
    /// AI inference endpoint template.
    pub const AI_RUN: &str = "/ai/run";
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

/// A client for the Cloudflare Workers AI API.
#[derive(Debug, Clone)]
pub struct CloudflareAiClient {
    inner: Arc<ClientInner>,
}

#[derive(Debug)]
struct ClientInner {
    http: reqwest::Client,
    account_id: String,
    #[allow(dead_code)]
    api_token: SecretString,
    base_url: String,
    max_retries: u32,
    retry_delay: Duration,
}

impl CloudflareAiClient {
    /// Create a new Cloudflare AI client with the given credentials.
    ///
    /// # Arguments
    ///
    /// * `account_id` - Your Cloudflare account ID
    /// * `api_token` - Your Cloudflare API token
    ///
    /// # Example
    ///
    /// ```
    /// use cloudflare_ai::CloudflareAiClient;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = CloudflareAiClient::new(
    ///     "your-account-id",
    ///     "your-api-token",
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(account_id: impl Into<String>, api_token: impl Into<SecretString>) -> Result<Self> {
        Self::with_config(ClientConfig::new(account_id, api_token))
    }

    /// Create a client with a custom configuration.
    pub fn with_config(config: ClientConfig) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", config.api_token.expose_secret()))
                .map_err(|_| CloudflareAiError::Config {
                    message: "Invalid API token".to_string(),
                })?,
        );

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(config.timeout)
            .build()
            .map_err(|e| CloudflareAiError::Config {
                message: format!("Failed to create HTTP client: {e}"),
            })?;

        Ok(Self {
            inner: Arc::new(ClientInner {
                http,
                account_id: config.account_id,
                api_token: config.api_token,
                base_url: config.base_url,
                max_retries: config.max_retries,
                retry_delay: config.retry_delay,
            }),
        })
    }

    /// Get the text generation API client.
    #[cfg(feature = "text")]
    pub fn text(&self) -> crate::text::Text<'_> {
        crate::text::Text::new(self)
    }

    /// Get the embeddings API client.
    #[cfg(feature = "embeddings")]
    pub fn embeddings(&self) -> crate::embeddings::Embeddings<'_> {
        crate::embeddings::Embeddings::new(self)
    }

    /// Get the translation API client.
    #[cfg(feature = "translation")]
    pub fn translation(&self) -> crate::translation::Translation<'_> {
        crate::translation::Translation::new(self)
    }

    /// Get the summarization API client.
    #[cfg(feature = "summarization")]
    pub fn summarization(&self) -> crate::summarization::Summarization<'_> {
        crate::summarization::Summarization::new(self)
    }

    /// Get the image classification API client.
    #[cfg(feature = "image")]
    pub fn image_classification(&self) -> crate::image::ImageClassification<'_> {
        crate::image::ImageClassification::new(self)
    }

    /// Get the text-to-image API client.
    #[cfg(feature = "image")]
    pub fn text_to_image(&self) -> crate::image::TextToImage<'_> {
        crate::image::TextToImage::new(self)
    }

    /// Get the speech recognition API client.
    #[cfg(feature = "speech")]
    pub fn speech(&self) -> crate::speech::Speech<'_> {
        crate::speech::Speech::new(self)
    }

    /// Get the account ID.
    pub fn account_id(&self) -> &str {
        &self.inner.account_id
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

        Err(CloudflareAiError::RetryExhausted {
            attempts,
            last_error: Box::new(last_error.unwrap()),
        })
    }

    /// Build the full URL for an AI model endpoint.
    pub(crate) fn build_url(&self, model: &str) -> String {
        format!(
            "{}/{}{}/{}",
            self.inner.base_url,
            self.inner.account_id,
            endpoints::AI_RUN,
            model
        )
    }

    /// Make a GET request to the API.
    #[allow(dead_code)]
    pub(crate) async fn get(&self, path: &str) -> Result<reqwest::Response> {
        let url = format!("{}/{}{}", self.inner.base_url, self.inner.account_id, path);
        trace!(url = %url, "Making GET request");

        let response = self
            .inner
            .http
            .get(&url)
            .send()
            .await
            .map_err(CloudflareAiError::from)?;

        Ok(response)
    }

    /// Make a POST request to the API.
    pub(crate) async fn post(
        &self,
        model: &str,
        body: serde_json::Value,
    ) -> Result<reqwest::Response> {
        let url = self.build_url(model);
        trace!(url = %url, body = %body, "Making POST request");

        let response = self
            .inner
            .http
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(CloudflareAiError::from)?;

        Ok(response)
    }

    /// Make a POST request with raw bytes body.
    #[allow(dead_code)]
    pub(crate) async fn post_bytes(
        &self,
        model: &str,
        body: Vec<u8>,
        content_type: &str,
    ) -> Result<reqwest::Response> {
        let url = self.build_url(model);
        trace!(url = %url, "Making POST request with bytes");

        let response = self
            .inner
            .http
            .post(&url)
            .header(CONTENT_TYPE, content_type)
            .body(body)
            .send()
            .await
            .map_err(CloudflareAiError::from)?;

        Ok(response)
    }

    /// Parse a response or return an error.
    pub(crate) async fn handle_response<T>(&self, response: reqwest::Response) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let status = response.status();

        if status.is_success() {
            let body = response.json::<T>().await.map_err(|e| {
                CloudflareAiError::Internal {
                    message: format!("Failed to parse JSON response: {e}"),
                }
            })?;
            Ok(body)
        } else if status == reqwest::StatusCode::NOT_FOUND {
            let body = response.text().await.unwrap_or_default();
            Err(CloudflareAiError::NotFound {
                resource: "model".to_string(),
                id: body,
            })
        } else {
            Err(CloudflareAiError::from_response(response).await)
        }
    }

    /// Parse a raw bytes response or return an error.
    #[allow(dead_code)]
    pub(crate) async fn handle_bytes_response(&self, response: reqwest::Response) -> Result<Vec<u8>> {
        let status = response.status();

        if status.is_success() {
            let body = response.bytes().await.map_err(CloudflareAiError::from)?;
            Ok(body.to_vec())
        } else {
            Err(CloudflareAiError::from_response(response).await)
        }
    }
}

/// Configuration for the Cloudflare AI client.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub(crate) account_id: String,
    pub(crate) api_token: SecretString,
    pub(crate) base_url: String,
    pub(crate) timeout: Duration,
    pub(crate) max_retries: u32,
    pub(crate) retry_delay: Duration,
}

impl ClientConfig {
    /// Create a new configuration with the given credentials.
    pub fn new(account_id: impl Into<String>, api_token: impl Into<SecretString>) -> Self {
        Self {
            account_id: account_id.into(),
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
        let config = ClientConfig::new("account123", "token456")
            .base_url("https://custom.api.com")
            .max_retries(5);

        assert_eq!(config.account_id, "account123");
        assert_eq!(config.api_token.expose_secret(), "token456");
        assert_eq!(config.base_url, "https://custom.api.com");
        assert_eq!(config.max_retries, 5);
    }

    #[test]
    fn test_client_creation() {
        let client = CloudflareAiClient::new("account123", "token456").unwrap();
        assert_eq!(client.base_url(), DEFAULT_BASE_URL);
        assert_eq!(client.account_id(), "account123");
    }

    #[test]
    fn test_build_url() {
        let client = CloudflareAiClient::new("account123", "token456").unwrap();
        let url = client.build_url("@cf/meta/llama-3-8b-instruct");
        assert_eq!(
            url,
            "https://api.cloudflare.com/client/v4/accounts/account123/ai/run/@cf/meta/llama-3-8b-instruct"
        );
    }
}
