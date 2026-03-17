//! HTTP client for the vLLM API.

use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use secrecy::{ExposeSecret, SecretString};

use crate::constants::DEFAULT_APP_NAME;
use crate::constants::retry::{MAX_DELAY_MS, MAX_RETRIES};
use crate::error::{Result, VllmError};

/// A client for the vLLM API.
#[derive(Debug, Clone)]
pub struct VllmClient {
    inner: Arc<ClientInner>,
}

#[derive(Debug)]
#[allow(dead_code)]
struct ClientInner {
    http: reqwest::Client,
    api_key: Option<SecretString>,
    base_url: String,
    app_name: String,
    max_retries: u32,
    retry_delay: Duration,
}

impl VllmClient {
    /// Create a new client with the given base URL.
    ///
    /// # Example
    ///
    /// ```
    /// use vllm::VllmClient;
    ///
    /// let client = VllmClient::new("http://localhost:8000").unwrap();
    /// ```
    pub fn new(base_url: impl Into<String>) -> Result<Self> {
        Self::with_config(ClientConfig::new(base_url))
    }

    /// Create a client with API key authentication.
    ///
    /// # Example
    ///
    /// ```
    /// use vllm::VllmClient;
    ///
    /// let client = VllmClient::with_api_key("http://localhost:8000", "sk-xxx").unwrap();
    /// ```
    pub fn with_api_key(
        base_url: impl Into<String>,
        api_key: impl Into<SecretString>,
    ) -> Result<Self> {
        let config = ClientConfig::new(base_url).with_api_key(api_key);
        Self::with_config(config)
    }

    /// Create a client with a custom configuration.
    pub fn with_config(config: ClientConfig) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        // Add authorization header if API key is provided
        if let Some(ref api_key) = config.api_key {
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {}", api_key.expose_secret())).map_err(
                    |_| VllmError::Config {
                        message: "Invalid API key".to_string(),
                    },
                )?,
            );
        }

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .connect_timeout(Duration::from_secs(10))
            .timeout(config.timeout)
            .build()
            .map_err(|e| VllmError::Config {
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

    /// Get the tokenize/detokenize API client.
    #[cfg(feature = "tokenize")]
    pub fn tokenize(&self) -> crate::tokenize::Tokenize<'_> {
        crate::tokenize::Tokenize::new(self)
    }

    /// Get the models API client.
    #[cfg(feature = "models")]
    pub fn models(&self) -> crate::models::Models<'_> {
        crate::models::Models::new(self)
    }

    /// Get the metrics API client.
    #[cfg(feature = "metrics")]
    pub fn metrics(&self) -> crate::metrics::Metrics<'_> {
        crate::metrics::Metrics::new(self)
    }

    /// Check the health of the vLLM server.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use vllm::VllmClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = VllmClient::new("http://localhost:8000")?;
    /// match client.health().await {
    ///     Ok(()) => println!("vLLM server is healthy"),
    ///     Err(e) => println!("Health check failed: {}", e),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn health(&self) -> Result<()> {
        use crate::constants::endpoints;

        let url = format!("{}{}", self.inner.base_url, endpoints::HEALTH);
        let response = self
            .inner
            .http
            .get(&url)
            .send()
            .await
            .map_err(VllmError::from)?;

        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Err(VllmError::Unhealthy {
                message: format!("HTTP {status}: {body}"),
            })
        }
    }

    /// Get the base URL.
    pub fn base_url(&self) -> &str {
        &self.inner.base_url
    }

    /// Get the API key (masked for security).
    pub fn api_key_masked(&self) -> Option<String> {
        self.inner.api_key.as_ref().map(|key| {
            let key = key.expose_secret();
            if key.len() <= 8 {
                "***".to_string()
            } else {
                format!("{}...{}", &key[..4], &key[key.len() - 4..])
            }
        })
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
                    delay = std::cmp::min(delay.mul_f64(2.0), Duration::from_millis(MAX_DELAY_MS));
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
            .map_err(VllmError::from)?;
        Ok(response)
    }

    /// Make a POST request.
    pub(crate) async fn post(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> Result<reqwest::Response> {
        let url = format!("{}{}", self.inner.base_url, path);
        let response = self
            .inner
            .http
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(VllmError::from)?;
        Ok(response)
    }

    /// Make a DELETE request.
    pub(crate) async fn _delete(&self, path: &str) -> Result<reqwest::Response> {
        let url = format!("{}{}", self.inner.base_url, path);
        let response = self
            .inner
            .http
            .delete(&url)
            .send()
            .await
            .map_err(VllmError::from)?;
        Ok(response)
    }

    /// Parse a response or return an error.
    pub(crate) async fn handle_response(
        &self,
        response: reqwest::Response,
    ) -> Result<serde_json::Value> {
        let status = response.status();

        if status.is_success() {
            let body =
                response
                    .json::<serde_json::Value>()
                    .await
                    .map_err(|e| VllmError::Internal {
                        message: format!("Failed to parse JSON response: {e}"),
                    })?;
            Ok(body)
        } else {
            Err(VllmError::from_response(response).await)
        }
    }

    /// Handle text response (for metrics endpoint).
    pub(crate) async fn handle_text_response(&self, response: reqwest::Response) -> Result<String> {
        let status = response.status();

        if status.is_success() {
            let body = response.text().await.map_err(|e| VllmError::Internal {
                message: format!("Failed to read text response: {e}"),
            })?;
            Ok(body)
        } else {
            Err(VllmError::from_response(response).await)
        }
    }

    /// Get the underlying HTTP client.
    #[allow(dead_code)]
    pub(crate) fn http(&self) -> &reqwest::Client {
        &self.inner.http
    }
}

/// Configuration for the vLLM client.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub(crate) api_key: Option<SecretString>,
    pub(crate) base_url: String,
    pub(crate) app_name: String,
    pub(crate) timeout: Duration,
    pub(crate) max_retries: u32,
    pub(crate) retry_delay: Duration,
}

impl ClientConfig {
    /// Create a new configuration.
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            api_key: None,
            base_url: base_url.into(),
            app_name: DEFAULT_APP_NAME.to_string(),
            timeout: Duration::from_secs(120),
            max_retries: MAX_RETRIES,
            retry_delay: Duration::from_millis(1000),
        }
    }

    /// Set the API key.
    pub fn with_api_key(mut self, api_key: impl Into<SecretString>) -> Self {
        self.api_key = Some(api_key.into());
        self
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

    /// Set the retry delay.
    pub fn retry_delay(mut self, delay: Duration) -> Self {
        self.retry_delay = delay;
        self
    }
}
