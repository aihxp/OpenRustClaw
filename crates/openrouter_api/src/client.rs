//! HTTP client for the OpenRouter API.

use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use secrecy::{ExposeSecret, SecretString};

use crate::constants::{retry, DEFAULT_APP_NAME, DEFAULT_BASE_URL, DEFAULT_REFERER};
use crate::error::{OpenRouterError, Result};

/// A client for the OpenRouter API.
#[derive(Debug, Clone)]
pub struct OpenRouterClient {
    inner: Arc<ClientInner>,
}

#[derive(Debug)]
struct ClientInner {
    http: reqwest::Client,
    _api_key: SecretString,
    base_url: String,
    _referer: String,
    _app_name: String,
    max_retries: u32,
    retry_delay: Duration,
}

impl OpenRouterClient {
    /// Create a new client with the given API key.
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
                .map_err(|_| OpenRouterError::Config {
                    message: "Invalid API key".to_string(),
                })?,
        );
        headers.insert(
            "HTTP-Referer",
            HeaderValue::from_str(&config.referer)
                .map_err(|_| OpenRouterError::Config {
                    message: "Invalid referer".to_string(),
                })?,
        );
        headers.insert(
            "X-Title",
            HeaderValue::from_str(&config.app_name)
                .map_err(|_| OpenRouterError::Config {
                    message: "Invalid app name".to_string(),
                })?,
        );

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .connect_timeout(Duration::from_secs(10))
            .timeout(config.timeout)
            .build()
            .map_err(|e| OpenRouterError::Config {
                message: format!("Failed to create HTTP client: {e}"),
            })?;

        Ok(Self {
            inner: Arc::new(ClientInner {
                http,
                _api_key: config.api_key,
                base_url: config.base_url,
                _referer: config.referer,
                _app_name: config.app_name,
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
            .map_err(OpenRouterError::from)?;
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
            .map_err(OpenRouterError::from)?;
        Ok(response)
    }

    /// Parse a response or return an error.
    pub(crate) async fn handle_response(&self, response: reqwest::Response) -> Result<serde_json::Value> {
        let status = response.status();

        if status.is_success() {
            let body = response.json::<serde_json::Value>().await.map_err(|e| {
                OpenRouterError::Internal {
                    message: format!("Failed to parse JSON response: {e}"),
                }
            })?;
            Ok(body)
        } else {
            Err(OpenRouterError::from_response(response).await)
        }
    }
}

/// Configuration for the OpenRouter client.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub(crate) api_key: SecretString,
    pub(crate) base_url: String,
    pub(crate) referer: String,
    pub(crate) app_name: String,
    pub(crate) timeout: Duration,
    pub(crate) max_retries: u32,
    pub(crate) retry_delay: Duration,
}

impl ClientConfig {
    /// Create a new configuration.
    pub fn new(api_key: impl Into<SecretString>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: DEFAULT_BASE_URL.to_string(),
            referer: DEFAULT_REFERER.to_string(),
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

    /// Set the HTTP referer.
    pub fn referer(mut self, referer: impl Into<String>) -> Self {
        self.referer = referer.into();
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
