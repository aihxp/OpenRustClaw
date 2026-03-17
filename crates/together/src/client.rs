//! HTTP client for the Together AI API.

use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};

use crate::constants::{retry, DEFAULT_APP_NAME, DEFAULT_BASE_URL};
use crate::error::{Result, TogetherError};

/// A client for the Together AI API.
#[derive(Debug, Clone)]
pub struct TogetherClient {
    inner: Arc<ClientInner>,
}

#[derive(Debug)]
#[allow(dead_code)]
struct ClientInner {
    http: reqwest::Client,
    api_key: String,
    base_url: String,
    app_name: String,
    max_retries: u32,
    retry_delay: Duration,
}

impl TogetherClient {
    /// Create a new client with the given API key.
    pub fn new(api_key: impl Into<String>) -> Result<Self> {
        Self::with_config(ClientConfig::new(api_key))
    }

    /// Create a client with a custom configuration.
    pub fn with_config(config: ClientConfig) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", config.api_key))
                .map_err(|_| TogetherError::Config {
                    message: "Invalid API key".to_string(),
                })?,
        );

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(config.timeout)
            .build()
            .map_err(|e| TogetherError::Config {
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
        let key = &self.inner.api_key;
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

        loop {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(err) => {
                    if !err.is_retryable() || attempts >= self.inner.max_retries {
                        return Err(err);
                    }

                    attempts += 1;
                    tokio::time::sleep(delay).await;
                    delay = std::cmp::min(
                        delay.mul_f64(2.0),
                        Duration::from_millis(retry::MAX_DELAY_MS),
                    );
                }
            }
        }
    }

    /// Make a GET request.
    pub(crate) async fn get(&self, path: &str) -> Result<reqwest::Response> {
        let url = format!("{}{}", self.inner.base_url, path);
        let response = self
            .inner
            .http
            .get(&url)
            .send()
            .await
            .map_err(TogetherError::from)?;
        Ok(response)
    }

    /// Make a POST request.
    pub(crate) async fn post(&self, path: &str, body: serde_json::Value) -> Result<reqwest::Response> {
        let url = format!("{}{}", self.inner.base_url, path);
        let response = self
            .inner
            .http
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(TogetherError::from)?;
        Ok(response)
    }

    /// Make a DELETE request.
    pub(crate) async fn delete(&self, path: &str) -> Result<reqwest::Response> {
        let url = format!("{}{}", self.inner.base_url, path);
        let response = self
            .inner
            .http
            .delete(&url)
            .send()
            .await
            .map_err(TogetherError::from)?;
        Ok(response)
    }

    /// Make a multipart POST request (for file uploads).
    pub(crate) async fn post_multipart(
        &self,
        path: &str,
        form: reqwest::multipart::Form,
    ) -> Result<reqwest::Response> {
        let url = format!("{}{}", self.inner.base_url, path);
        let response = self
            .inner
            .http
            .post(&url)
            .multipart(form)
            .send()
            .await
            .map_err(TogetherError::from)?;
        Ok(response)
    }

    /// Parse a response or return an error.
    pub(crate) async fn handle_response(&self, response: reqwest::Response) -> Result<serde_json::Value> {
        let status = response.status();

        if status.is_success() {
            let body = response.json::<serde_json::Value>().await.map_err(|e| {
                TogetherError::Internal {
                    message: format!("Failed to parse JSON response: {e}"),
                }
            })?;
            Ok(body)
        } else {
            Err(TogetherError::from_response(response).await)
        }
    }

    /// Get the underlying HTTP client.
    #[allow(dead_code)]
    pub(crate) fn http(&self) -> &reqwest::Client {
        &self.inner.http
    }
}

/// Configuration for the Together AI client.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub(crate) api_key: String,
    pub(crate) base_url: String,
    pub(crate) app_name: String,
    pub(crate) timeout: Duration,
    pub(crate) max_retries: u32,
    pub(crate) retry_delay: Duration,
}

impl ClientConfig {
    /// Create a new configuration.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: DEFAULT_BASE_URL.to_string(),
            app_name: DEFAULT_APP_NAME.to_string(),
            timeout: Duration::from_secs(120),
            max_retries: retry::MAX_RETRIES,
            retry_delay: Duration::from_millis(retry::INITIAL_DELAY_MS),
        }
    }

    /// Set the base URL.
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Set the app name.
    pub fn app_name(mut self, name: impl Into<String>) -> Self {
        self.app_name = name.into();
        self
    }

    /// Set the timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the max retries.
    pub fn max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }
}
