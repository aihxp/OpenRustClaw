//! HTTP client for E2E tests.

use reqwest::{Client, Method, RequestBuilder, Response};
use serde::{de::DeserializeOwned, Serialize};
use std::time::Duration;

/// HTTP client for E2E testing
pub struct TestHttpClient {
    client: Client,
    base_url: String,
    _default_timeout: Duration,
}

impl TestHttpClient {
    /// Create a new test HTTP client
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
            base_url: base_url.into(),
            _default_timeout: Duration::from_secs(30),
        }
    }

    /// Create a new test HTTP client with custom timeout
    pub fn with_timeout(base_url: impl Into<String>, timeout_secs: u64) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(timeout_secs))
                .build()
                .expect("Failed to create HTTP client"),
            base_url: base_url.into(),
            _default_timeout: Duration::from_secs(timeout_secs),
        }
    }

    /// Build a request URL
    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    /// Make a GET request
    pub async fn get(&self, path: &str) -> reqwest::Result<Response> {
        self.client.get(self.url(path)).send().await
    }

    /// Make a GET request with query parameters
    pub async fn get_with_query<Q: Serialize + ?Sized>(
        &self,
        path: &str,
        query: &Q,
    ) -> reqwest::Result<Response> {
        self.client
            .get(self.url(path))
            .query(query)
            .send()
            .await
    }

    /// Make a POST request with JSON body
    pub async fn post<B: Serialize>(&self, path: &str, body: &B) -> reqwest::Result<Response> {
        self.client
            .post(self.url(path))
            .json(body)
            .send()
            .await
    }

    /// Make a PUT request with JSON body
    pub async fn put<B: Serialize>(&self, path: &str, body: &B) -> reqwest::Result<Response> {
        self.client
            .put(self.url(path))
            .json(body)
            .send()
            .await
    }

    /// Make a DELETE request
    pub async fn delete(&self, path: &str) -> reqwest::Result<Response> {
        self.client.delete(self.url(path)).send().await
    }

    /// Make a request with custom method and body
    pub fn request<B: Serialize>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
    ) -> RequestBuilder {
        let mut builder = self.client.request(method, self.url(path));
        if let Some(b) = body {
            builder = builder.json(b);
        }
        builder
    }

    /// Parse response body as JSON
    pub async fn parse_json<T: DeserializeOwned>(response: Response) -> Result<T, TestError> {
        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| TestError::Http(e.to_string()))?;

        if !status.is_success() {
            return Err(TestError::Http(format!(
                "HTTP {}: {}",
                status.as_u16(),
                text
            )));
        }

        serde_json::from_str(&text).map_err(|e| TestError::Json(e))
    }

    /// Check if server is healthy
    pub async fn health_check(&self) -> Result<bool, TestError> {
        match self.get("/health").await {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    /// Wait for server to be ready
    pub async fn wait_for_ready(&self, timeout_secs: u64) -> Result<(), TestError> {
        let start = std::time::Instant::now();
        let timeout = Duration::from_secs(timeout_secs);

        while start.elapsed() < timeout {
            if self.health_check().await? {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        Err(TestError::Timeout(format!(
            "Server not ready after {} seconds",
            timeout_secs
        )))
    }
}

#[derive(Debug)]
pub enum TestError {
    Http(String),
    Json(serde_json::Error),
    Timeout(String),
}

impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestError::Http(msg) => write!(f, "HTTP error: {}", msg),
            TestError::Json(e) => write!(f, "JSON error: {}", e),
            TestError::Timeout(msg) => write!(f, "Timeout: {}", msg),
        }
    }
}

impl std::error::Error for TestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TestError::Json(e) => Some(e),
            _ => None,
        }
    }
}

/// WebSocket client for real-time testing
pub struct TestWebSocketClient {
    url: String,
}

impl TestWebSocketClient {
    /// Create a new WebSocket test client
    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into() }
    }

    /// Connect to WebSocket endpoint
    pub async fn connect(
        &self,
    ) -> Result<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, TestError> {
        use tokio_tungstenite::connect_async;

        let (ws_stream, _) = connect_async(&self.url)
            .await
            .map_err(|e| TestError::Http(format!("WebSocket connection failed: {}", e)))?;

        Ok(ws_stream)
    }
}

/// Response assertions
pub trait ResponseExt {
    fn assert_status(&self, expected: u16) -> &Self;
    fn assert_success(&self) -> &Self;
    fn assert_content_type(&self, expected: &str) -> &Self;
}

impl ResponseExt for Response {
    fn assert_status(&self, expected: u16) -> &Self {
        let actual = self.status().as_u16();
        assert_eq!(
            actual, expected,
            "Expected status {}, but got {}",
            expected, actual
        );
        self
    }

    fn assert_success(&self) -> &Self {
        assert!(
            self.status().is_success(),
            "Expected success status, but got {}",
            self.status()
        );
        self
    }

    fn assert_content_type(&self, expected: &str) -> &Self {
        let actual = self
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("missing");
        assert!(
            actual.contains(expected),
            "Expected content-type containing '{}', but got '{}'",
            expected,
            actual
        );
        self
    }
}
