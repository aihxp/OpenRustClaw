//! HTTP client for the Anthropic API.

use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use tracing::{debug, trace};

use crate::constants::{headers, retry, DEFAULT_API_VERSION, DEFAULT_BASE_URL};
use crate::error::{AnthropicError, Result};
use crate::types::{MessageRequest, MessageResponse};

#[cfg(feature = "streaming")]
use crate::streaming::StreamEvent;

/// A client for the Anthropic API.
#[derive(Debug, Clone)]
pub struct AnthropicClient {
    inner: Arc<ClientInner>,
}

#[derive(Debug)]
struct ClientInner {
    http: reqwest::Client,
    api_key: String,
    base_url: String,
    api_version: String,
    max_retries: u32,
    retry_delay: Duration,
}

impl AnthropicClient {
    /// Create a new Anthropic client with the given API key.
    pub fn new(api_key: impl Into<String>) -> Result<Self> {
        Self::with_config(ClientConfig::new(api_key))
    }

    /// Create a client with a custom configuration.
    pub fn with_config(config: ClientConfig) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            headers::X_API_KEY,
            HeaderValue::from_str(&config.api_key)
                .map_err(|_| AnthropicError::Config {
                    message: "Invalid API key".to_string(),
                })?,
        );
        headers.insert(
            headers::ANTHROPIC_VERSION,
            HeaderValue::from_str(&config.api_version).unwrap_or_else(|_| HeaderValue::from_static(DEFAULT_API_VERSION)),
        );

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(config.timeout)
            .build()
            .map_err(|e| AnthropicError::Config {
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

    /// Get the Messages API client.
    pub fn messages(&self) -> Messages<'_> {
        Messages { client: self }
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
    async fn execute_with_retry<T, F>(&self, operation: F) -> Result<T>
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

        Err(AnthropicError::RetryExhausted {
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
            .map_err(AnthropicError::from)?;

        Ok(response)
    }

    /// Parse a response or return an error.
    pub(crate) async fn handle_response(&self, response: reqwest::Response) -> Result<serde_json::Value> {
        let status = response.status();

        if status.is_success() {
            let body = response.json::<serde_json::Value>().await.map_err(|e| {
                AnthropicError::Internal {
                    message: format!("Failed to parse JSON response: {e}"),
                }
            })?;
            Ok(body)
        } else {
            Err(AnthropicError::from_response(response).await)
        }
    }
}

/// Client for the Messages API.
#[derive(Debug)]
pub struct Messages<'a> {
    client: &'a AnthropicClient,
}

impl<'a> Messages<'a> {
    /// Send a message request and get a complete response.
    pub async fn create(&self, request: MessageRequest) -> Result<MessageResponse> {
        let body = serde_json::to_value(&request)?;

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let body = body.clone();
                Box::pin(async move {
                    let response = client.post(crate::constants::endpoints::MESSAGES, body).await?;
                    let body = client.handle_response(response).await?;
                    let message: MessageResponse = serde_json::from_value(body)?;
                    Ok(message)
                })
            })
            .await
    }

    /// Send a streaming message request.
    #[cfg(feature = "streaming")]
    pub async fn stream(
        &self,
        request: MessageRequest,
    ) -> Result<impl futures::Stream<Item = Result<StreamEvent>>> {
        use eventsource_stream::Eventsource;
        use futures::StreamExt;

        let mut body = serde_json::to_value(&request)?;
        body["stream"] = serde_json::json!(true);

        let url = format!(
            "{}{}",
            self.client.inner.base_url,
            crate::constants::endpoints::MESSAGES
        );

        debug!("Initiating streaming request");

        let response = self
            .client
            .inner
            .http
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(AnthropicError::from)?;

        let status = response.status();
        if !status.is_success() {
            return Err(AnthropicError::from_response(response).await);
        }

        let stream = response
            .bytes_stream()
            .eventsource()
            .map(|event| {
                match event {
                    Ok(event) => {
                        if event.data == "[DONE]" {
                            return Ok(StreamEvent::MessageStop);
                        }

                        match serde_json::from_str::<StreamEvent>(&event.data) {
                            Ok(stream_event) => Ok(stream_event),
                            Err(e) => Err(AnthropicError::Stream {
                                message: format!("Failed to parse SSE event: {e}"),
                            }),
                        }
                    }
                    Err(e) => Err(AnthropicError::Stream {
                        message: format!("SSE error: {e}"),
                    }),
                }
            })
            .filter(|result| {
                // Filter out ping events
                let should_keep = !matches!(result, Ok(StreamEvent::Ping));
                std::future::ready(should_keep)
            });

        Ok(stream)
    }

    /// Count tokens in a message (if the API supports it).
    pub async fn count_tokens(&self, _request: MessageRequest) -> Result<usize> {
        // Note: Anthropic doesn't have a dedicated token counting endpoint
        // This is a placeholder that could use tiktoken or similar
        Err(AnthropicError::Internal {
            message: "Token counting not implemented".to_string(),
        })
    }
}

/// Configuration for the Anthropic client.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    api_key: String,
    base_url: String,
    api_version: String,
    timeout: Duration,
    max_retries: u32,
    retry_delay: Duration,
}

impl ClientConfig {
    /// Create a new configuration with the given API key.
    pub fn new(api_key: impl Into<String>) -> Self {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_config_builder() {
        let config = ClientConfig::new("test-key")
            .base_url("https://custom.api.com")
            .api_version("2023-06-01")
            .max_retries(5);

        assert_eq!(config.base_url, "https://custom.api.com");
        assert_eq!(config.max_retries, 5);
    }

    #[test]
    fn test_client_creation() {
        let client = AnthropicClient::new("test-key").unwrap();
        assert_eq!(client.base_url(), DEFAULT_BASE_URL);
    }
}
