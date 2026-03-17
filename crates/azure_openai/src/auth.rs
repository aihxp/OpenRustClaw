//! Authentication support for Azure OpenAI.
//!
//! This module provides various authentication methods for Azure OpenAI Service:
//! - API Key authentication
//! - Azure AD Token authentication
//! - Managed Identity authentication

use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;

use crate::error::{AzureOpenAIError, Result};

/// A trait for token credentials.
#[async_trait::async_trait]
pub trait TokenCredential: Send + Sync {
    /// Get an access token.
    async fn get_token(&self) -> Result<String>;

    /// Get the token with cache support.
    async fn get_token_with_cache(&self) -> Result<String>;
}

/// Azure AD Token for authentication.
#[derive(Debug, Clone)]
pub struct AzureADToken {
    /// The token value.
    token: String,
    /// Client ID for client credentials flow.
    client_id: Option<String>,
    /// Client secret for client credentials flow.
    client_secret: Option<String>,
    /// Tenant ID for client credentials flow.
    tenant_id: Option<String>,
}

impl AzureADToken {
    /// Create a new Azure AD token with the given token string.
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            token: token.into(),
            client_id: None,
            client_secret: None,
            tenant_id: None,
        }
    }

    /// Create a new Azure AD token with client credentials for automatic token refresh.
    pub fn with_client_credentials(
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        tenant_id: impl Into<String>,
    ) -> Self {
        Self {
            token: String::new(),
            client_id: Some(client_id.into()),
            client_secret: Some(client_secret.into()),
            tenant_id: Some(tenant_id.into()),
        }
    }

    /// Get the token.
    pub async fn get_token(&self) -> Result<String> {
        if !self.token.is_empty() {
            Ok(self.token.clone())
        } else if let (Some(client_id), Some(client_secret), Some(tenant_id)) =
            (&self.client_id, &self.client_secret, &self.tenant_id)
        {
            // Acquire token using client credentials flow
            Self::acquire_token_client_credentials(client_id, client_secret, tenant_id).await
        } else {
            Err(AzureOpenAIError::TokenAcquisition {
                message: "No token or client credentials provided".to_string(),
            })
        }
    }

    /// Acquire a token using the client credentials flow.
    async fn acquire_token_client_credentials(
        client_id: &str,
        client_secret: &str,
        tenant_id: &str,
    ) -> Result<String> {
        let url = format!(
            "https://login.microsoftonline.com/{}/oauth2/v2.0/token",
            tenant_id
        );

        let params = [
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("scope", "https://cognitiveservices.azure.com/.default"),
            ("grant_type", "client_credentials"),
        ];

        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .form(&params)
            .send()
            .await
            .map_err(|e| AzureOpenAIError::TokenAcquisition {
                message: format!("Failed to acquire token: {e}"),
            })?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(AzureOpenAIError::TokenAcquisition {
                message: format!("Token acquisition failed: {error_text}"),
            });
        }

        let token_response: TokenResponse = response.json().await.map_err(|e| {
            AzureOpenAIError::TokenAcquisition {
                message: format!("Failed to parse token response: {e}"),
            }
        })?;

        Ok(token_response.access_token)
    }
}

/// Token response from Azure AD.
#[derive(Debug, Clone, serde::Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
    token_type: String,
}

/// Managed Identity credential for Azure resources.
#[derive(Debug, Clone)]
pub struct ManagedIdentityCredential {
    /// The client ID of the managed identity (optional for system-assigned).
    client_id: Option<String>,
    /// The resource/scope to request token for.
    resource: String,
}

impl ManagedIdentityCredential {
    /// Create a new managed identity credential for system-assigned identity.
    pub fn new() -> Self {
        Self {
            client_id: None,
            resource: "https://cognitiveservices.azure.com/".to_string(),
        }
    }

    /// Create a new managed identity credential with a specific client ID (user-assigned).
    pub fn with_client_id(client_id: impl Into<String>) -> Self {
        Self {
            client_id: Some(client_id.into()),
            resource: "https://cognitiveservices.azure.com/".to_string(),
        }
    }

    /// Create a new managed identity credential with a custom resource.
    pub fn with_resource(resource: impl Into<String>) -> Self {
        Self {
            client_id: None,
            resource: resource.into(),
        }
    }

    /// Get a token using managed identity.
    pub async fn get_token(&self) -> Result<String> {
        // Try IMDS (Instance Metadata Service) first
        let imds_url = format!(
            "http://169.254.169.254/metadata/identity/oauth2/token?api-version=2018-02-01&resource={}",
            urlencoding::encode(&self.resource)
        );

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| AzureOpenAIError::TokenAcquisition {
                message: format!("Failed to create HTTP client: {e}"),
            })?;

        let request = client.get(&imds_url).header("Metadata", "true");

        let request = if let Some(client_id) = &self.client_id {
            request.query(&[("client_id", client_id)])
        } else {
            request
        };

        let response = request.send().await.map_err(|e| {
            AzureOpenAIError::TokenAcquisition {
                message: format!("IMDS request failed: {e}"),
            }
        })?;

        if !response.status().is_success() {
            // Fall back to MSI_ENDPOINT for App Service/Functions
            return self.get_token_msi_endpoint().await;
        }

        let token_response: ManagedIdentityTokenResponse = response.json().await.map_err(|e| {
            AzureOpenAIError::TokenAcquisition {
                message: format!("Failed to parse token response: {e}"),
            }
        })?;

        Ok(token_response.access_token)
    }

    /// Get token using MSI_ENDPOINT (for App Service/Azure Functions).
    async fn get_token_msi_endpoint(&self) -> Result<String> {
        let msi_endpoint = std::env::var("MSI_ENDPOINT").map_err(|_| {
            AzureOpenAIError::TokenAcquisition {
                message: "MSI_ENDPOINT not set and IMDS failed".to_string(),
            }
        })?;

        let msi_secret = std::env::var("MSI_SECRET").map_err(|_| {
            AzureOpenAIError::TokenAcquisition {
                message: "MSI_SECRET not set".to_string(),
            }
        })?;

        let client = reqwest::Client::new();
        let request = client
            .get(&msi_endpoint)
            .header("Secret", msi_secret)
            .query(&[("resource", &self.resource)]);

        let request = if let Some(client_id) = &self.client_id {
            request.query(&[("clientid", client_id)])
        } else {
            request
        };

        let response = request.send().await.map_err(|e| {
            AzureOpenAIError::TokenAcquisition {
                message: format!("MSI request failed: {e}"),
            }
        })?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(AzureOpenAIError::TokenAcquisition {
                message: format!("MSI token acquisition failed: {error_text}"),
            });
        }

        let token_response: ManagedIdentityTokenResponse = response.json().await.map_err(|e| {
            AzureOpenAIError::TokenAcquisition {
                message: format!("Failed to parse token response: {e}"),
            }
        })?;

        Ok(token_response.access_token)
    }
}

impl Default for ManagedIdentityCredential {
    fn default() -> Self {
        Self::new()
    }
}

/// Token response from managed identity endpoint.
#[derive(Debug, Clone, serde::Deserialize)]
struct ManagedIdentityTokenResponse {
    access_token: String,
    expires_on: Option<String>,
    resource: Option<String>,
    token_type: Option<String>,
}

/// Cached token with expiration tracking.
#[derive(Debug, Clone)]
struct CachedToken {
    token: String,
    expires_at: Instant,
}

/// Token cache for automatic refresh.
pub struct TokenCache {
    credential: Arc<dyn TokenCredential + Send + Sync>,
    cached_token: Arc<RwLock<Option<CachedToken>>>,
    refresh_buffer: Duration,
}

impl std::fmt::Debug for TokenCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenCache")
            .field("refresh_buffer", &self.refresh_buffer)
            .finish()
    }
}

impl Clone for TokenCache {
    fn clone(&self) -> Self {
        Self {
            credential: Arc::clone(&self.credential),
            cached_token: Arc::clone(&self.cached_token),
            refresh_buffer: self.refresh_buffer,
        }
    }
}

impl TokenCache {
    /// Create a new token cache with the given credential.
    pub fn new<T>(credential: T) -> Self
    where
        T: TokenCredential + Send + Sync + 'static,
    {
        Self {
            credential: Arc::new(credential),
            cached_token: Arc::new(RwLock::new(None)),
            refresh_buffer: Duration::from_secs(300), // Refresh 5 minutes before expiry
        }
    }

    /// Get a token, using cache if available and not expired.
    pub async fn get_token(&self) -> Result<String> {
        // Check if we have a valid cached token
        {
            let cache = self.cached_token.read().await;
            if let Some(ref cached) = *cache {
                if cached.expires_at > Instant::now() + self.refresh_buffer {
                    return Ok(cached.token.clone());
                }
            }
        }

        // Acquire new token
        let token = self.credential.get_token().await?;

        // Cache the token with a default expiration of 1 hour
        let expires_at = Instant::now() + Duration::from_secs(3600);
        {
            let mut cache = self.cached_token.write().await;
            *cache = Some(CachedToken { token, expires_at });
        }

        // Return the token
        let cache = self.cached_token.read().await;
        Ok(cache.as_ref().unwrap().token.clone())
    }
}

/// Chained token credential that tries multiple credentials in order.
pub struct ChainedTokenCredential {
    credentials: Vec<Arc<dyn TokenCredential + Send + Sync>>,
}

impl std::fmt::Debug for ChainedTokenCredential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChainedTokenCredential")
            .field("credentials_count", &self.credentials.len())
            .finish()
    }
}

impl ChainedTokenCredential {
    /// Create a new chained credential.
    pub fn new() -> Self {
        Self {
            credentials: Vec::new(),
        }
    }

    /// Add a credential to the chain.
    pub fn push<T>(mut self, credential: T) -> Self
    where
        T: TokenCredential + Send + Sync + 'static,
    {
        self.credentials.push(Arc::new(credential));
        self
    }
}

impl Default for ChainedTokenCredential {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl TokenCredential for ChainedTokenCredential {
    async fn get_token(&self) -> Result<String> {
        for credential in &self.credentials {
            match credential.get_token().await {
                Ok(token) => return Ok(token),
                Err(_) => continue,
            }
        }
        Err(AzureOpenAIError::TokenAcquisition {
            message: "All credentials in the chain failed".to_string(),
        })
    }

    async fn get_token_with_cache(&self) -> Result<String> {
        self.get_token().await
    }
}

/// Default Azure credential that tries common authentication methods.
///
/// This tries the following methods in order:
/// 1. Environment variables (AZURE_CLIENT_ID, AZURE_CLIENT_SECRET, AZURE_TENANT_ID)
/// 2. Managed Identity
/// 3. Azure CLI
pub struct DefaultAzureCredential;

impl DefaultAzureCredential {
    /// Create a new default Azure credential.
    pub fn new() -> Self {
        Self
    }

    /// Get a token using the default credential chain.
    pub async fn get_token() -> Result<String> {
        // Try environment-based client credentials first
        if let (Ok(client_id), Ok(client_secret), Ok(tenant_id)) = (
            std::env::var("AZURE_CLIENT_ID"),
            std::env::var("AZURE_CLIENT_SECRET"),
            std::env::var("AZURE_TENANT_ID"),
        ) {
            return AzureADToken::acquire_token_client_credentials(
                &client_id,
                &client_secret,
                &tenant_id,
            )
            .await;
        }

        // Try managed identity
        let managed_identity = ManagedIdentityCredential::new();
        match managed_identity.get_token().await {
            Ok(token) => return Ok(token),
            Err(_) => {}
        }

        Err(AzureOpenAIError::TokenAcquisition {
            message: "No valid credential found in the chain".to_string(),
        })
    }
}

impl Default for DefaultAzureCredential {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl TokenCredential for DefaultAzureCredential {
    async fn get_token(&self) -> Result<String> {
        Self::get_token().await
    }

    async fn get_token_with_cache(&self) -> Result<String> {
        self.get_token().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_azure_ad_token_new() {
        let token = AzureADToken::new("test-token");
        // Cannot directly test async get_token without runtime
    }

    #[test]
    fn test_managed_identity_credential() {
        let cred = ManagedIdentityCredential::new();
        assert!(cred.client_id.is_none());

        let cred = ManagedIdentityCredential::with_client_id("client-123");
        assert_eq!(cred.client_id, Some("client-123".to_string()));
    }

    #[test]
    fn test_chained_credential() {
        let chain = ChainedTokenCredential::new();
        assert!(chain.credentials.is_empty());
    }
}
