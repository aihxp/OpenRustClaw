//! Configuration for the Azure OpenAI client.

use std::time::Duration;

use crate::constants::{retry, DEFAULT_API_VERSION};
use crate::error::{AzureOpenAIError, Result};
use crate::{AzureADToken, AzureRegion, ManagedIdentityCredential};

/// Azure OpenAI client configuration.
#[derive(Debug, Clone)]
pub struct AzureConfig {
    /// The Azure OpenAI resource name.
    pub resource_name: String,
    /// The deployment name.
    pub deployment_name: String,
    /// The API version.
    pub api_version: String,
    /// The Azure region (optional, for specific regional endpoints).
    pub region: Option<AzureRegion>,
    /// The credential to use for authentication.
    pub credential: AzureCredential,
    /// Request timeout.
    pub timeout: Duration,
    /// Maximum number of retries.
    pub max_retries: u32,
    /// Initial retry delay.
    pub retry_delay: Duration,
}

/// Azure authentication credential.
#[derive(Debug, Clone)]
pub enum AzureCredential {
    /// API key authentication.
    ApiKey(String),
    /// Azure AD token authentication.
    AzureADToken(AzureADToken),
    /// Managed Identity authentication.
    #[cfg(feature = "managed-identity")]
    ManagedIdentity(ManagedIdentityCredential),
    /// Bearer token (for external token acquisition).
    BearerToken(String),
}

impl AzureConfig {
    /// Create a new configuration for API key authentication.
    pub fn api_key(api_key: impl Into<String>) -> Self {
        Self {
            resource_name: String::new(),
            deployment_name: String::new(),
            api_version: DEFAULT_API_VERSION.to_string(),
            region: None,
            credential: AzureCredential::ApiKey(api_key.into()),
            timeout: Duration::from_secs(120),
            max_retries: retry::MAX_RETRIES,
            retry_delay: Duration::from_millis(retry::INITIAL_DELAY_MS),
        }
    }

    /// Create a new configuration for Azure AD token authentication.
    pub fn azure_ad_token(token: impl Into<String>) -> Self {
        Self {
            resource_name: String::new(),
            deployment_name: String::new(),
            api_version: DEFAULT_API_VERSION.to_string(),
            region: None,
            credential: AzureCredential::AzureADToken(AzureADToken::new(token)),
            timeout: Duration::from_secs(120),
            max_retries: retry::MAX_RETRIES,
            retry_delay: Duration::from_millis(retry::INITIAL_DELAY_MS),
        }
    }

    /// Create a new configuration for Azure AD token authentication with client credentials.
    pub fn azure_ad_client_credentials(
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        tenant_id: impl Into<String>,
    ) -> Self {
        Self {
            resource_name: String::new(),
            deployment_name: String::new(),
            api_version: DEFAULT_API_VERSION.to_string(),
            region: None,
            credential: AzureCredential::AzureADToken(AzureADToken::with_client_credentials(
                client_id,
                client_secret,
                tenant_id,
            )),
            timeout: Duration::from_secs(120),
            max_retries: retry::MAX_RETRIES,
            retry_delay: Duration::from_millis(retry::INITIAL_DELAY_MS),
        }
    }

    /// Create a new configuration with a bearer token.
    pub fn bearer_token(token: impl Into<String>) -> Self {
        Self {
            resource_name: String::new(),
            deployment_name: String::new(),
            api_version: DEFAULT_API_VERSION.to_string(),
            region: None,
            credential: AzureCredential::BearerToken(token.into()),
            timeout: Duration::from_secs(120),
            max_retries: retry::MAX_RETRIES,
            retry_delay: Duration::from_millis(retry::INITIAL_DELAY_MS),
        }
    }

    /// Create a new configuration for Managed Identity authentication.
    #[cfg(feature = "managed-identity")]
    pub fn managed_identity() -> Self {
        Self {
            resource_name: String::new(),
            deployment_name: String::new(),
            api_version: DEFAULT_API_VERSION.to_string(),
            region: None,
            credential: AzureCredential::ManagedIdentity(ManagedIdentityCredential::new()),
            timeout: Duration::from_secs(120),
            max_retries: retry::MAX_RETRIES,
            retry_delay: Duration::from_millis(retry::INITIAL_DELAY_MS),
        }
    }

    /// Create a new configuration for Managed Identity authentication with a specific client ID.
    #[cfg(feature = "managed-identity")]
    pub fn managed_identity_with_client_id(client_id: impl Into<String>) -> Self {
        Self {
            resource_name: String::new(),
            deployment_name: String::new(),
            api_version: DEFAULT_API_VERSION.to_string(),
            region: None,
            credential: AzureCredential::ManagedIdentity(
                ManagedIdentityCredential::with_client_id(client_id),
            ),
            timeout: Duration::from_secs(120),
            max_retries: retry::MAX_RETRIES,
            retry_delay: Duration::from_millis(retry::INITIAL_DELAY_MS),
        }
    }

    /// Set the resource name.
    pub fn resource_name(mut self, name: impl Into<String>) -> Self {
        self.resource_name = name.into();
        self
    }

    /// Set the deployment name.
    pub fn deployment_name(mut self, name: impl Into<String>) -> Self {
        self.deployment_name = name.into();
        self
    }

    /// Set the API version.
    pub fn api_version(mut self, version: impl Into<String>) -> Self {
        self.api_version = version.into();
        self
    }

    /// Set the Azure region.
    pub fn region(mut self, region: AzureRegion) -> Self {
        self.region = Some(region);
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

    /// Build the base URL for the Azure OpenAI resource.
    pub fn base_url(&self) -> String {
        crate::constants::build_base_url(&self.resource_name)
    }

    /// Validate the configuration.
    pub fn validate(&self) -> Result<()> {
        if self.resource_name.is_empty() {
            return Err(AzureOpenAIError::Config {
                message: "Resource name is required".to_string(),
            });
        }

        if self.deployment_name.is_empty() {
            return Err(AzureOpenAIError::Config {
                message: "Deployment name is required".to_string(),
            });
        }

        Ok(())
    }

    /// Get the authorization header value.
    pub(crate) async fn authorization_header(&self) -> Result<String> {
        match &self.credential {
            AzureCredential::ApiKey(key) => Ok(format!("Bearer {key}")),
            AzureCredential::AzureADToken(token_credential) => {
                let token = token_credential.get_token().await?;
                Ok(format!("Bearer {token}"))
            }
            #[cfg(feature = "managed-identity")]
            AzureCredential::ManagedIdentity(credential) => {
                let token = credential.get_token().await?;
                Ok(format!("Bearer {token}"))
            }
            AzureCredential::BearerToken(token) => Ok(format!("Bearer {token}")),
        }
    }

    /// Check if this configuration uses Azure AD authentication.
    pub fn is_azure_ad(&self) -> bool {
        matches!(
            self.credential,
            AzureCredential::AzureADToken(_)
        )
    }

    /// Check if this configuration uses API key authentication.
    pub fn is_api_key(&self) -> bool {
        matches!(self.credential, AzureCredential::ApiKey(_))
    }
}

impl Default for AzureConfig {
    fn default() -> Self {
        Self {
            resource_name: String::new(),
            deployment_name: String::new(),
            api_version: DEFAULT_API_VERSION.to_string(),
            region: None,
            credential: AzureCredential::ApiKey(String::new()),
            timeout: Duration::from_secs(120),
            max_retries: retry::MAX_RETRIES,
            retry_delay: Duration::from_millis(retry::INITIAL_DELAY_MS),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_builder() {
        let config = AzureConfig::api_key("test-key")
            .resource_name("my-resource")
            .deployment_name("my-deployment")
            .api_version("2024-06-01")
            .region(AzureRegion::EastUS)
            .max_retries(5);

        assert_eq!(config.resource_name, "my-resource");
        assert_eq!(config.deployment_name, "my-deployment");
        assert_eq!(config.api_version, "2024-06-01");
        assert_eq!(config.region, Some(AzureRegion::EastUS));
        assert_eq!(config.max_retries, 5);
        assert!(config.is_api_key());
    }

    #[test]
    fn test_config_base_url() {
        let config = AzureConfig::api_key("test-key")
            .resource_name("my-resource")
            .deployment_name("my-deployment");

        assert_eq!(config.base_url(), "https://my-resource.openai.azure.com");
    }

    #[test]
    fn test_config_validation() {
        let config = AzureConfig::api_key("test-key");
        assert!(config.validate().is_err());

        let config = AzureConfig::api_key("test-key")
            .resource_name("my-resource")
            .deployment_name("my-deployment");
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_azure_ad_config() {
        let config = AzureConfig::azure_ad_token("test-token")
            .resource_name("my-resource")
            .deployment_name("my-deployment");

        assert!(config.is_azure_ad());
    }
}
