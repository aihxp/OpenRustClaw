//! AWS Bedrock client.
//!
//! The main entry point for the AWS Bedrock SDK.

use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use tracing::{debug, trace};

use crate::auth::{
    service_endpoint, AwsCredentials, CredentialChain, Region, Service,
    SigV4Signer,
};
use crate::constants::{DEFAULT_MAX_RETRIES, DEFAULT_RETRY_DELAY, DEFAULT_TIMEOUT};
use crate::converse::ConverseClient;
use crate::error::{BedrockError, Result};

/// AWS Bedrock client.
#[derive(Debug, Clone)]
pub struct BedrockClient {
    inner: Arc<ClientInner>,
}

#[derive(Debug)]
struct ClientInner {
    http: reqwest::Client,
    region: Region,
    _credentials: AwsCredentials,
    signer: SigV4Signer,
    converse: ConverseClient,
}

impl BedrockClient {
    /// Create a new Bedrock client with the default credential chain.
    ///
    /// This will attempt to resolve credentials from:
    /// 1. Environment variables (AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY)
    /// 2. Shared credentials file (~/.aws/credentials)
    /// 3. Container credentials (ECS/EKS)
    /// 4. EC2 instance metadata service (IMDS)
    pub async fn new(region: impl Into<Region>) -> Result<Self> {
        let config = ClientConfig::new(region);
        Self::with_config(config).await
    }

    /// Create a client with a specific configuration.
    pub async fn with_config(config: ClientConfig) -> Result<Self> {
        trace!("Creating Bedrock client");

        // Resolve credentials
        let credentials = match config.credentials {
            Some(creds) => creds,
            None => {
                CredentialChain::default_chain()
                    .resolve()
                    .await
                    .map_err(|e| BedrockError::Credential {
                        message: format!("Failed to resolve credentials: {e}"),
                    })?
            }
        };

        debug!("Credentials resolved successfully");

        // Build HTTP client
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(config.timeout)
            .build()
            .map_err(|e| BedrockError::Config {
                message: format!("Failed to create HTTP client: {e}"),
            })?;

        // Create signer
        let signer = SigV4Signer::new(config.region.clone(), credentials.clone());

        // Create Converse client
        let converse = ConverseClient::new(http.clone(), config.region.clone(), signer.clone());

        Ok(Self {
            inner: Arc::new(ClientInner {
                http,
                region: config.region,
                _credentials: credentials,
                signer,
                converse,
            }),
        })
    }

    /// Create a client with explicit credentials.
    pub async fn with_credentials(
        region: impl Into<Region>,
        credentials: AwsCredentials,
    ) -> Result<Self> {
        let config = ClientConfig::new(region).with_credentials(credentials);
        Self::with_config(config).await
    }

    /// Create a client from environment variables.
    pub async fn from_env(region: impl Into<Region>) -> Result<Self> {
        let config = ClientConfig::from_env(region)?;
        Self::with_config(config).await
    }

    /// Create a client with a profile.
    pub async fn with_profile(
        region: impl Into<Region>,
        profile: impl Into<String>,
    ) -> Result<Self> {
        let config = ClientConfig::with_profile(region, profile)?;
        Self::with_config(config).await
    }

    /// Get the Converse API client.
    pub fn converse(&self) -> &ConverseClient {
        &self.inner.converse
    }

    /// Get the region.
    pub fn region(&self) -> &Region {
        &self.inner.region
    }

    /// Get the runtime endpoint URL.
    pub fn runtime_endpoint(&self) -> String {
        service_endpoint(Service::BedrockRuntime, &self.inner.region)
    }

    /// Get the control plane endpoint URL.
    pub fn endpoint(&self) -> String {
        service_endpoint(Service::Bedrock, &self.inner.region)
    }

    /// Make a signed request to the Bedrock API.
    pub async fn request(
        &self,
        method: &str,
        path: &str,
        body: Option<serde_json::Value>,
        service: Service,
    ) -> Result<reqwest::Response> {
        let endpoint = service_endpoint(service, &self.inner.region);
        let url = format!("{}{}", endpoint, path);

        trace!(method = %method, url = %url, "Making API request");

        // Sign the request
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let body_bytes = body
            .as_ref()
            .map(|b| b.to_string().into_bytes())
            .unwrap_or_default();

        self.inner.signer.sign_request(
            method,
            &url,
            &mut headers,
            &body_bytes,
            service,
        )?;

        // Build and send request
        let mut request_builder = self.inner.http.request(
            reqwest::Method::from_bytes(method.as_bytes()).map_err(|e| BedrockError::Config {
                message: format!("Invalid HTTP method: {e}"),
            })?,
            &url,
        );

        request_builder = request_builder.headers(headers);

        if let Some(body) = body {
            request_builder = request_builder.json(&body);
        }

        let response = request_builder.send().await.map_err(BedrockError::from)?;

        Ok(response)
    }

    /// Execute a request with retry logic.
    pub async fn execute_with_retry<T, F>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send>>,
    {
        let mut attempts = 0;
        let mut delay = DEFAULT_RETRY_DELAY;
        let mut last_error = None;

        while attempts <= DEFAULT_MAX_RETRIES {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(err) => {
                    if !err.is_retryable() || attempts == DEFAULT_MAX_RETRIES {
                        return Err(err);
                    }

                    if let Some(retry_after) = err.retry_after() {
                        delay = retry_after;
                    }

                    last_error = Some(err);
                    attempts += 1;

                    debug!(
                        attempt = attempts,
                        max_retries = DEFAULT_MAX_RETRIES,
                        delay = ?delay,
                        "Retrying request"
                    );

                    tokio::time::sleep(delay).await;
                    delay = std::cmp::min(
                        delay.mul_f64(2.0),
                        Duration::from_millis(32000),
                    );
                }
            }
        }

        Err(BedrockError::RetryExhausted {
            attempts,
            last_error: Box::new(last_error.unwrap()),
        })
    }
}

/// Configuration for the Bedrock client.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// AWS region.
    pub region: Region,
    /// AWS credentials (if None, will use credential chain).
    pub credentials: Option<AwsCredentials>,
    /// Request timeout.
    pub timeout: Duration,
    /// Maximum retries.
    pub max_retries: u32,
    /// Retry delay.
    pub retry_delay: Duration,
}

impl ClientConfig {
    /// Create a new configuration for the given region.
    pub fn new(region: impl Into<Region>) -> Self {
        Self {
            region: region.into(),
            credentials: None,
            timeout: DEFAULT_TIMEOUT,
            max_retries: DEFAULT_MAX_RETRIES,
            retry_delay: DEFAULT_RETRY_DELAY,
        }
    }

    /// Set explicit credentials.
    pub fn with_credentials(mut self, credentials: AwsCredentials) -> Self {
        self.credentials = Some(credentials);
        self
    }

    /// Set the request timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the maximum number of retries.
    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Set the retry delay.
    pub fn retry_delay(mut self, retry_delay: Duration) -> Self {
        self.retry_delay = retry_delay;
        self
    }

    /// Create configuration from environment variables.
    pub fn from_env(region: impl Into<Region>) -> Result<Self> {
        use crate::auth::EnvironmentCredentialProvider;
        use crate::auth::CredentialProvider;

        let rt = tokio::runtime::Runtime::new().map_err(|e| BedrockError::Config {
            message: format!("Failed to create runtime: {e}"),
        })?;

        let provider = EnvironmentCredentialProvider::new();
        let credentials = rt.block_on(provider.provide_credentials())?;

        Ok(Self::new(region).with_credentials(credentials))
    }

    /// Create configuration with a profile.
    pub fn with_profile(region: impl Into<Region>, profile: impl Into<String>) -> Result<Self> {
        use crate::auth::ProfileCredentialProvider;
        use crate::auth::CredentialProvider;

        let rt = tokio::runtime::Runtime::new().map_err(|e| BedrockError::Config {
            message: format!("Failed to create runtime: {e}"),
        })?;

        let provider = ProfileCredentialProvider::new(profile);
        let credentials = rt.block_on(provider.provide_credentials())?;

        Ok(Self::new(region).with_credentials(credentials))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_config() {
        let config = ClientConfig::new("us-east-1")
            .max_retries(5)
            .timeout(Duration::from_secs(60));

        assert_eq!(config.region.name(), "us-east-1");
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_client_config_with_credentials() {
        let creds = AwsCredentials::new("AKIAIOSFODNN7EXAMPLE", "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY");
        let config = ClientConfig::new("us-west-2").with_credentials(creds);

        assert!(config.credentials.is_some());
    }
}
