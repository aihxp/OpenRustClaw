//! HTTP client for the Azure OpenAI API.

use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use secrecy::{ExposeSecret, SecretString};
use tracing::{debug, trace};

use crate::config::AzureConfig;
use crate::constants::retry;
use crate::error::{AzureOpenAIError, Result};

/// A client for the Azure OpenAI API.
#[derive(Debug, Clone)]
pub struct AzureOpenAIClient {
    inner: Arc<ClientInner>,
}

#[derive(Debug)]
struct ClientInner {
    http: reqwest::Client,
    config: AzureConfig,
}

impl AzureOpenAIClient {
    /// Create a new Azure OpenAI client with API key authentication.
    ///
    /// # Arguments
    ///
    /// * `resource_name` - The name of your Azure OpenAI resource
    /// * `deployment_name` - The name of your model deployment
    /// * `api_key` - Your Azure OpenAI API key
    ///
    /// # Example
    ///
    /// ```
    /// use azure_openai::AzureOpenAIClient;
    ///
    /// let client = AzureOpenAIClient::new(
    ///     "my-resource",
    ///     "my-deployment",
    ///     "my-api-key",
    /// ).unwrap();
    /// ```
    pub fn new(
        resource_name: impl Into<String>,
        deployment_name: impl Into<String>,
        api_key: impl Into<SecretString>,
    ) -> Result<Self> {
        let config = AzureConfig::api_key(api_key)
            .resource_name(resource_name)
            .deployment_name(deployment_name);

        Self::with_config(config)
    }

    /// Create a client with a custom configuration.
    ///
    /// # Example
    ///
    /// ```
    /// use azure_openai::{AzureOpenAIClient, AzureConfig};
    ///
    /// let config = AzureConfig::api_key("my-api-key")
    ///     .resource_name("my-resource")
    ///     .deployment_name("my-deployment");
    ///
    /// let client = AzureOpenAIClient::with_config(config).unwrap();
    /// ```
    pub fn with_config(config: AzureConfig) -> Result<Self> {
        config.validate()?;

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        // Note: Authorization header will be added per-request for Azure AD
        // For API key auth, we add it here
        if config.is_api_key() {
            if let crate::AzureCredential::ApiKey(key) = &config.credential {
                headers.insert(
                    "api-key",
                    HeaderValue::from_str(key.expose_secret()).map_err(|_| AzureOpenAIError::Config {
                        message: "Invalid API key".to_string(),
                    })?,
                );
            }
        }

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(config.timeout)
            .build()
            .map_err(|e| AzureOpenAIError::Config {
                message: format!("Failed to create HTTP client: {e}"),
            })?;

        Ok(Self {
            inner: Arc::new(ClientInner { http, config }),
        })
    }

    /// Create a client with Azure AD token authentication.
    ///
    /// # Arguments
    ///
    /// * `resource_name` - The name of your Azure OpenAI resource
    /// * `deployment_name` - The name of your model deployment
    /// * `token` - Your Azure AD access token
    ///
    /// # Example
    ///
    /// ```
    /// use azure_openai::AzureOpenAIClient;
    ///
    /// let client = AzureOpenAIClient::with_azure_ad_token(
    ///     "my-resource",
    ///     "my-deployment",
    ///     "my-aad-token",
    /// ).unwrap();
    /// ```
    pub fn with_azure_ad_token(
        resource_name: impl Into<String>,
        deployment_name: impl Into<String>,
        token: impl Into<String>,
    ) -> Result<Self> {
        let config = AzureConfig::azure_ad_token(token)
            .resource_name(resource_name)
            .deployment_name(deployment_name);

        Self::with_config(config)
    }

    /// Get the chat completions API client.
    pub fn chat(&self) -> crate::chat::Chat<'_> {
        crate::chat::Chat::new(self)
    }

    /// Get the completions API client.
    #[cfg(feature = "completions")]
    pub fn completions(&self) -> crate::completions::Completions<'_> {
        crate::completions::Completions::new(self)
    }

    /// Get the embeddings API client.
    pub fn embeddings(&self) -> crate::embeddings::Embeddings<'_> {
        crate::embeddings::Embeddings::new(self)
    }

    /// Get the images API client.
    #[cfg(feature = "images")]
    pub fn images(&self) -> crate::images::Images<'_> {
        crate::images::Images::new(self)
    }

    /// Get the audio API client.
    #[cfg(feature = "audio")]
    pub fn audio(&self) -> crate::audio::Audio<'_> {
        crate::audio::Audio::new(self)
    }

    /// Get the assistants API client.
    #[cfg(feature = "assistants")]
    pub fn assistants(&self) -> crate::assistants::Assistants<'_> {
        crate::assistants::Assistants::new(self)
    }

    /// Get the batch API client.
    #[cfg(feature = "batch")]
    pub fn batch(&self) -> crate::batch::Batch<'_> {
        crate::batch::Batch::new(self)
    }

    /// Get the files API client.
    #[cfg(feature = "files")]
    pub fn files(&self) -> crate::files::Files<'_> {
        crate::files::Files::new(self)
    }

    /// Get the fine-tuning API client.
    #[cfg(feature = "fine-tuning")]
    pub fn fine_tuning(&self) -> crate::fine_tuning::FineTuning<'_> {
        crate::fine_tuning::FineTuning::new(self)
    }

    /// Get the base URL.
    pub fn base_url(&self) -> String {
        self.inner.config.base_url()
    }

    /// Get the resource name.
    pub fn resource_name(&self) -> &str {
        &self.inner.config.resource_name
    }

    /// Get the deployment name.
    pub fn deployment_name(&self) -> &str {
        &self.inner.config.deployment_name
    }

    /// Get the API version.
    pub fn api_version(&self) -> &str {
        &self.inner.config.api_version
    }

    /// Get the configuration.
    pub fn config(&self) -> &AzureConfig {
        &self.inner.config
    }

    /// Check if using Azure AD authentication.
    pub fn is_azure_ad(&self) -> bool {
        self.inner.config.is_azure_ad()
    }

    /// Build a URL with the API version query parameter.
    pub(crate) fn build_url(&self, path: &str) -> String {
        format!(
            "{}{}?api-version={}",
            self.base_url(),
            path,
            self.api_version()
        )
    }

    /// Get the HTTP client.
    pub(crate) fn http(&self) -> &reqwest::Client {
        &self.inner.http
    }

    /// Execute a request with retry logic.
    pub(crate) async fn execute_with_retry<T, F>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send>>,
    {
        let mut attempts = 0;
        let mut delay = self.inner.config.retry_delay;
        let mut last_error = None;

        while attempts <= self.inner.config.max_retries {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(err) => {
                    if !err.is_retryable() || attempts == self.inner.config.max_retries {
                        return Err(err);
                    }

                    if let Some(retry_after) = err.retry_after() {
                        delay = retry_after;
                    }

                    last_error = Some(err);
                    attempts += 1;

                    debug!(
                        attempt = attempts,
                        max_retries = self.inner.config.max_retries,
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

        Err(AzureOpenAIError::RetryExhausted {
            attempts,
            last_error: Box::new(last_error.unwrap()),
        })
    }

    /// Get authorization headers for the request.
    pub(crate) async fn auth_headers(&self) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();

        if self.is_azure_ad() {
            let auth_header = self.inner.config.authorization_header().await?;
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&auth_header).map_err(|_| AzureOpenAIError::Config {
                    message: "Invalid authorization header".to_string(),
                })?,
            );
        }

        Ok(headers)
    }

    /// Make a GET request to the API.
    pub(crate) async fn get(&self, path: &str) -> Result<reqwest::Response> {
        let url = self.build_url(path);
        trace!(url = %url, "Making GET request");

        let mut request = self.inner.http.get(&url);

        // Add auth headers for Azure AD
        if self.is_azure_ad() {
            let headers = self.auth_headers().await?;
            for (key, value) in headers.iter() {
                request = request.header(key, value);
            }
        }

        let response = request.send().await.map_err(AzureOpenAIError::from)?;
        Ok(response)
    }

    /// Make a POST request to the API.
    pub(crate) async fn post(&self, path: &str, body: serde_json::Value) -> Result<reqwest::Response> {
        let url = self.build_url(path);
        trace!(url = %url, body = %body, "Making POST request");

        let mut request = self.inner.http.post(&url).json(&body);

        // Add auth headers for Azure AD
        if self.is_azure_ad() {
            let headers = self.auth_headers().await?;
            for (key, value) in headers.iter() {
                request = request.header(key, value);
            }
        }

        let response = request.send().await.map_err(AzureOpenAIError::from)?;
        Ok(response)
    }

    /// Make a DELETE request to the API.
    pub(crate) async fn delete(&self, path: &str) -> Result<reqwest::Response> {
        let url = self.build_url(path);
        trace!(url = %url, "Making DELETE request");

        let mut request = self.inner.http.delete(&url);

        // Add auth headers for Azure AD
        if self.is_azure_ad() {
            let headers = self.auth_headers().await?;
            for (key, value) in headers.iter() {
                request = request.header(key, value);
            }
        }

        let response = request.send().await.map_err(AzureOpenAIError::from)?;
        Ok(response)
    }

    /// Parse a response or return an error.
    pub(crate) async fn handle_response(&self, response: reqwest::Response) -> Result<serde_json::Value> {
        let status = response.status();

        if status.is_success() {
            let body = response.json::<serde_json::Value>().await.map_err(|e| {
                AzureOpenAIError::Internal {
                    message: format!("Failed to parse JSON response: {e}"),
                }
            })?;
            Ok(body)
        } else if status == reqwest::StatusCode::NOT_FOUND {
            let body = response.text().await.unwrap_or_default();
            Err(AzureOpenAIError::NotFound {
                resource: "resource".to_string(),
                id: body,
            })
        } else {
            Err(AzureOpenAIError::from_response(response).await)
        }
    }

    /// Make a multipart POST request (for file uploads).
    #[cfg(any(feature = "audio", feature = "files"))]
    pub(crate) async fn post_multipart(
        &self,
        path: &str,
        form: reqwest::multipart::Form,
    ) -> Result<reqwest::Response> {
        let url = self.build_url(path);
        trace!(url = %url, "Making multipart POST request");

        let mut request = self.inner.http.post(&url).multipart(form);

        // Add auth headers for Azure AD
        if self.is_azure_ad() {
            let headers = self.auth_headers().await?;
            for (key, value) in headers.iter() {
                request = request.header(key, value);
            }
        }

        let response = request.send().await.map_err(AzureOpenAIError::from)?;
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = AzureOpenAIClient::new(
            "test-resource",
            "test-deployment",
            "test-api-key",
        )
        .unwrap();

        assert_eq!(client.resource_name(), "test-resource");
        assert_eq!(client.deployment_name(), "test-deployment");
        assert_eq!(client.base_url(), "https://test-resource.openai.azure.com");
    }

    #[test]
    fn test_client_config_builder() {
        let config = AzureConfig::api_key("test-key")
            .resource_name("my-resource")
            .deployment_name("my-deployment")
            .api_version("2024-06-01");

        let client = AzureOpenAIClient::with_config(config).unwrap();
        assert_eq!(client.api_version(), "2024-06-01");
    }

    #[test]
    fn test_client_azure_ad() {
        let client = AzureOpenAIClient::with_azure_ad_token(
            "test-resource",
            "test-deployment",
            "test-token",
        )
        .unwrap();

        assert!(client.is_azure_ad());
    }

    #[test]
    fn test_build_url() {
        let client = AzureOpenAIClient::new(
            "test-resource",
            "test-deployment",
            "test-api-key",
        )
        .unwrap();

        let url = client.build_url("/openai/deployments/test-deployment/chat/completions");
        assert!(url.contains("api-version="));
        assert!(url.contains("test-resource.openai.azure.com"));
    }
}
