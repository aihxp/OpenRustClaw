//! AWS credential handling.
//!
//! This module provides credential providers that follow the standard
//! AWS credential chain resolution order.

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

use tracing::{debug, trace};

use crate::error::{BedrockError, Result};

/// AWS credentials.
#[derive(Debug, Clone)]
pub struct AwsCredentials {
    /// AWS access key ID.
    access_key_id: String,
    /// AWS secret access key.
    secret_access_key: String,
    /// Optional session token for temporary credentials.
    session_token: Option<String>,
    /// Expiration time for temporary credentials.
    expires_at: Option<SystemTime>,
}

impl AwsCredentials {
    /// Create new AWS credentials.
    pub fn new(
        access_key_id: impl Into<String>,
        secret_access_key: impl Into<String>,
    ) -> Self {
        Self {
            access_key_id: access_key_id.into(),
            secret_access_key: secret_access_key.into(),
            session_token: None,
            expires_at: None,
        }
    }

    /// Create new temporary credentials with a session token.
    pub fn with_session_token(
        access_key_id: impl Into<String>,
        secret_access_key: impl Into<String>,
        session_token: impl Into<String>,
        expires_at: Option<SystemTime>,
    ) -> Self {
        Self {
            access_key_id: access_key_id.into(),
            secret_access_key: secret_access_key.into(),
            session_token: Some(session_token.into()),
            expires_at,
        }
    }

    /// Get the access key ID.
    pub fn access_key_id(&self) -> &str {
        &self.access_key_id
    }

    /// Get the secret access key.
    pub fn secret_access_key(&self) -> &str {
        &self.secret_access_key
    }

    /// Get the session token if present.
    pub fn session_token(&self) -> Option<&str> {
        self.session_token.as_deref()
    }

    /// Check if these credentials have expired.
    pub fn is_expired(&self) -> bool {
        match self.expires_at {
            Some(expires) => SystemTime::now() >= expires,
            None => false,
        }
    }

    /// Check if these are temporary credentials.
    pub fn is_temporary(&self) -> bool {
        self.session_token.is_some()
    }
}

/// Trait for credential providers.
#[async_trait::async_trait]
pub trait CredentialProvider: Send + Sync {
    /// Provide credentials.
    async fn provide_credentials(&self) -> Result<AwsCredentials>;

    /// Check if this provider is valid (can provide credentials).
    fn is_valid(&self) -> bool {
        true
    }
}

/// Static credential provider.
#[derive(Debug, Clone)]
pub struct StaticCredentialProvider {
    credentials: AwsCredentials,
}

impl StaticCredentialProvider {
    /// Create a new static credential provider.
    pub fn new(credentials: AwsCredentials) -> Self {
        Self { credentials }
    }

    /// Create from individual components.
    pub fn from_keys(
        access_key_id: impl Into<String>,
        secret_access_key: impl Into<String>,
    ) -> Self {
        Self::new(AwsCredentials::new(access_key_id, secret_access_key))
    }
}

#[async_trait::async_trait]
impl CredentialProvider for StaticCredentialProvider {
    async fn provide_credentials(&self) -> Result<AwsCredentials> {
        if self.credentials.is_expired() {
            return Err(BedrockError::Credential {
                message: "Credentials have expired".to_string(),
            });
        }
        Ok(self.credentials.clone())
    }
}

/// Environment variable credential provider.
#[derive(Debug, Clone, Default)]
pub struct EnvironmentCredentialProvider;

impl EnvironmentCredentialProvider {
    /// Create a new environment credential provider.
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl CredentialProvider for EnvironmentCredentialProvider {
    async fn provide_credentials(&self) -> Result<AwsCredentials> {
        trace!("Checking environment for AWS credentials");

        let access_key_id = env::var("AWS_ACCESS_KEY_ID")
            .or_else(|_| env::var("AWS_ACCESS_KEY"))
            .map_err(|_| BedrockError::Credential {
                message: "AWS_ACCESS_KEY_ID not set".to_string(),
            })?;

        let secret_access_key = env::var("AWS_SECRET_ACCESS_KEY")
            .or_else(|_| env::var("AWS_SECRET_KEY"))
            .map_err(|_| BedrockError::Credential {
                message: "AWS_SECRET_ACCESS_KEY not set".to_string(),
            })?;

        let session_token = env::var("AWS_SESSION_TOKEN").ok();

        debug!("Found credentials in environment");

        Ok(AwsCredentials {
            access_key_id,
            secret_access_key,
            session_token,
            expires_at: None,
        })
    }

    fn is_valid(&self) -> bool {
        env::var("AWS_ACCESS_KEY_ID").is_ok() || env::var("AWS_ACCESS_KEY").is_ok()
    }
}

/// Profile credential provider (from AWS config files).
#[derive(Debug, Clone)]
pub struct ProfileCredentialProvider {
    profile_name: String,
    config_path: Option<PathBuf>,
    credentials_path: Option<PathBuf>,
}

impl ProfileCredentialProvider {
    /// Create a new profile credential provider.
    pub fn new(profile_name: impl Into<String>) -> Self {
        Self {
            profile_name: profile_name.into(),
            config_path: None,
            credentials_path: None,
        }
    }

    /// Use the default profile.
    pub fn default_profile() -> Self {
        Self::new("default")
    }

    /// Set a custom config file path.
    pub fn with_config_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.config_path = Some(path.into());
        self
    }

    /// Set a custom credentials file path.
    pub fn with_credentials_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.credentials_path = Some(path.into());
        self
    }

    /// Get the default AWS config file path.
    #[allow(dead_code)]
    fn default_config_path() -> Option<PathBuf> {
        env::var("AWS_CONFIG_FILE")
            .ok()
            .map(PathBuf::from)
            .or_else(|| {
                home::home_dir().map(|home| home.join(".aws").join("config"))
            })
    }

    /// Get the default AWS credentials file path.
    fn default_credentials_path() -> Option<PathBuf> {
        env::var("AWS_SHARED_CREDENTIALS_FILE")
            .ok()
            .map(PathBuf::from)
            .or_else(|| {
                home::home_dir().map(|home| home.join(".aws").join("credentials"))
            })
    }

    /// Parse an AWS config file.
    #[allow(dead_code)]
    fn parse_config_file(&self, path: &PathBuf) -> Result<HashMap<String, HashMap<String, String>>> {
        let content = fs::read_to_string(path)
            .map_err(|e| BedrockError::Credential {
                message: format!("Failed to read config file: {e}"),
            })?;

        let mut profiles = HashMap::new();
        let mut current_profile: Option<String> = None;
        let mut current_settings: HashMap<String, String> = HashMap::new();

        for line in content.lines() {
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }

            // Parse profile header [profile name] or [default]
            if line.starts_with('[') && line.ends_with(']') {
                // Save previous profile if exists
                if let Some(profile) = current_profile.take() {
                    profiles.insert(profile, current_settings.clone());
                }

                let profile_name = line[1..line.len() - 1].trim();
                let profile_name = if profile_name.starts_with("profile ") {
                    profile_name[8..].to_string()
                } else {
                    profile_name.to_string()
                };

                current_profile = Some(profile_name);
                current_settings.clear();
            } else if let Some(eq_pos) = line.find('=') {
                let key = line[..eq_pos].trim().to_string();
                let value = line[eq_pos + 1..].trim().to_string();
                current_settings.insert(key, value);
            }
        }

        // Save the last profile
        if let Some(profile) = current_profile {
            profiles.insert(profile, current_settings);
        }

        Ok(profiles)
    }

    /// Parse an AWS credentials file.
    fn parse_credentials_file(
        &self,
        path: &PathBuf,
    ) -> Result<HashMap<String, HashMap<String, String>>> {
        let content = fs::read_to_string(path)
            .map_err(|e| BedrockError::Credential {
                message: format!("Failed to read credentials file: {e}"),
            })?;

        let mut profiles = HashMap::new();
        let mut current_profile: Option<String> = None;
        let mut current_credentials: HashMap<String, String> = HashMap::new();

        for line in content.lines() {
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }

            // Parse profile header [name]
            if line.starts_with('[') && line.ends_with(']') {
                // Save previous profile if exists
                if let Some(profile) = current_profile.take() {
                    profiles.insert(profile, current_credentials.clone());
                }

                let profile_name = line[1..line.len() - 1].trim().to_string();
                current_profile = Some(profile_name);
                current_credentials.clear();
            } else if let Some(eq_pos) = line.find('=') {
                let key = line[..eq_pos].trim().to_string();
                let value = line[eq_pos + 1..].trim().to_string();
                current_credentials.insert(key, value);
            }
        }

        // Save the last profile
        if let Some(profile) = current_profile {
            profiles.insert(profile, current_credentials);
        }

        Ok(profiles)
    }
}

#[async_trait::async_trait]
impl CredentialProvider for ProfileCredentialProvider {
    async fn provide_credentials(&self) -> Result<AwsCredentials> {
        trace!(profile = %self.profile_name, "Loading credentials from profile");

        let credentials_path = self
            .credentials_path
            .clone()
            .or_else(|| Self::default_credentials_path())
            .ok_or_else(|| BedrockError::Credential {
                message: "Could not determine credentials file path".to_string(),
            })?;

        let profiles = self.parse_credentials_file(&credentials_path)?;

        let credentials = profiles.get(&self.profile_name).ok_or_else(|| {
            BedrockError::Credential {
                message: format!("Profile '{}' not found in credentials file", self.profile_name),
            }
        })?;

        let access_key_id = credentials
            .get("aws_access_key_id")
            .cloned()
            .ok_or_else(|| BedrockError::Credential {
                message: format!("aws_access_key_id not found for profile '{}'", self.profile_name),
            })?;

        let secret_access_key = credentials
            .get("aws_secret_access_key")
            .cloned()
            .ok_or_else(|| BedrockError::Credential {
                message: format!(
                    "aws_secret_access_key not found for profile '{}'",
                    self.profile_name
                ),
            })?;

        let session_token = credentials.get("aws_session_token").cloned();

        debug!(profile = %self.profile_name, "Loaded credentials from profile");

        Ok(AwsCredentials {
            access_key_id,
            secret_access_key,
            session_token,
            expires_at: None,
        })
    }

    fn is_valid(&self) -> bool {
        let credentials_path = self
            .credentials_path
            .clone()
            .or_else(|| Self::default_credentials_path());

        credentials_path.map(|p| p.exists()).unwrap_or(false)
    }
}

/// Container credentials provider (for ECS/EKS).
#[derive(Debug, Clone, Default)]
pub struct ContainerCredentialProvider;

impl ContainerCredentialProvider {
    /// Create a new container credential provider.
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl CredentialProvider for ContainerCredentialProvider {
    async fn provide_credentials(&self) -> Result<AwsCredentials> {
        let relative_uri = env::var("AWS_CONTAINER_CREDENTIALS_RELATIVE_URI").map_err(|_| {
            BedrockError::Credential {
                message: "Not running in container environment".to_string(),
            }
        })?;

        // ECS provides the full endpoint, EKS provides relative URI
        let endpoint = if let Ok(full_uri) = env::var("AWS_CONTAINER_CREDENTIALS_FULL_URI") {
            full_uri
        } else {
            format!("http://169.254.170.2{}", relative_uri)
        };

        trace!("Fetching credentials from container endpoint");

        let client = reqwest::Client::new();
        let response = client
            .get(&endpoint)
            .send()
            .await
            .map_err(|e| BedrockError::Credential {
                message: format!("Failed to fetch container credentials: {e}"),
            })?;

        let creds_json: serde_json::Value = response.json().await.map_err(|e| {
            BedrockError::Credential {
                message: format!("Failed to parse container credentials: {e}"),
            }
        })?;

        let access_key_id = creds_json["AccessKeyId"]
            .as_str()
            .ok_or_else(|| BedrockError::Credential {
                message: "AccessKeyId not found in container credentials".to_string(),
            })?;

        let secret_access_key = creds_json["SecretAccessKey"]
            .as_str()
            .ok_or_else(|| BedrockError::Credential {
                message: "SecretAccessKey not found in container credentials".to_string(),
            })?;

        let session_token = creds_json["Token"].as_str().map(|s| s.to_string());

        let expires_at = creds_json["Expiration"]
            .as_str()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| SystemTime::from(dt));

        debug!("Loaded credentials from container endpoint");

        Ok(AwsCredentials {
            access_key_id: access_key_id.to_string(),
            secret_access_key: secret_access_key.to_string(),
            session_token,
            expires_at,
        })
    }

    fn is_valid(&self) -> bool {
        env::var("AWS_CONTAINER_CREDENTIALS_RELATIVE_URI").is_ok()
            || env::var("AWS_CONTAINER_CREDENTIALS_FULL_URI").is_ok()
    }
}

/// EC2 instance metadata service (IMDS) credential provider.
#[derive(Debug, Clone)]
pub struct InstanceMetadataCredentialProvider {
    endpoint: String,
    version: ImdsVersion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImdsVersion {
    V1,
    V2,
}

impl InstanceMetadataCredentialProvider {
    /// Create a new IMDS credential provider.
    pub fn new() -> Self {
        Self {
            endpoint: "http://169.254.169.254".to_string(),
            version: ImdsVersion::V2,
        }
    }

    /// Set the IMDS endpoint.
    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = endpoint.into();
        self
    }

    /// Use IMDSv1 instead of v2.
    pub fn use_v1(mut self) -> Self {
        self.version = ImdsVersion::V1;
        self
    }

    /// Get IMDSv2 token.
    async fn get_token(&self, client: &reqwest::Client) -> Result<String> {
        let token_url = format!("{}/latest/api/token", self.endpoint);

        let response = client
            .put(&token_url)
            .header("X-aws-ec2-metadata-token-ttl-seconds", "300")
            .send()
            .await
            .map_err(|e| BedrockError::Credential {
                message: format!("Failed to get IMDSv2 token: {e}"),
            })?;

        let token = response.text().await.map_err(|e| BedrockError::Credential {
            message: format!("Failed to read IMDSv2 token: {e}"),
        })?;

        Ok(token)
    }
}

impl Default for InstanceMetadataCredentialProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl CredentialProvider for InstanceMetadataCredentialProvider {
    async fn provide_credentials(&self) -> Result<AwsCredentials> {
        trace!("Fetching credentials from EC2 instance metadata");

        let client = reqwest::Client::new();

        let mut request = client
            .get(format!(
                "{}/latest/meta-data/iam/security-credentials/",
                self.endpoint
            ));

        // Use IMDSv2 token if available
        let token = if self.version == ImdsVersion::V2 {
            self.get_token(&client).await.ok()
        } else {
            None
        };

        if let Some(token) = &token {
            request = request.header("X-aws-ec2-metadata-token", token);
        }

        let role_response = request.send().await.map_err(|e| BedrockError::Credential {
            message: format!("Failed to fetch role name from IMDS: {e}"),
        })?;

        let role_name = role_response.text().await.map_err(|e| {
            BedrockError::Credential {
                message: format!("Failed to read role name from IMDS: {e}"),
            }
        })?;

        // Fetch credentials for the role
        let mut creds_request = client.get(format!(
            "{}/latest/meta-data/iam/security-credentials/{}",
            self.endpoint, role_name
        ));

        if let Some(token) = &token {
            creds_request = creds_request.header("X-aws-ec2-metadata-token", token);
        }

        let creds_response = creds_request.send().await.map_err(|e| {
            BedrockError::Credential {
                message: format!("Failed to fetch credentials from IMDS: {e}"),
            }
        })?;

        let creds_json: serde_json::Value = creds_response.json().await.map_err(|e| {
            BedrockError::Credential {
                message: format!("Failed to parse IMDS credentials: {e}"),
            }
        })?;

        let access_key_id = creds_json["AccessKeyId"]
            .as_str()
            .ok_or_else(|| BedrockError::Credential {
                message: "AccessKeyId not found in IMDS credentials".to_string(),
            })?;

        let secret_access_key = creds_json["SecretAccessKey"]
            .as_str()
            .ok_or_else(|| BedrockError::Credential {
                message: "SecretAccessKey not found in IMDS credentials".to_string(),
            })?;

        let session_token = creds_json["Token"].as_str().map(|s| s.to_string());

        let expires_at = creds_json["Expiration"]
            .as_str()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| SystemTime::from(dt));

        debug!("Loaded credentials from EC2 instance metadata");

        Ok(AwsCredentials {
            access_key_id: access_key_id.to_string(),
            secret_access_key: secret_access_key.to_string(),
            session_token,
            expires_at,
        })
    }

    fn is_valid(&self) -> bool {
        // IMDS is available on EC2 instances
        // Check for EC2-specific environment variable or hypervisor uuid
        env::var("EC2_INSTANCE_ID").is_ok() || self.is_ec2_instance()
    }
}

impl InstanceMetadataCredentialProvider {
    fn is_ec2_instance(&self) -> bool {
        // Simple heuristic: check if we can reach the IMDS endpoint
        // In practice, this is done by checking /sys/hypervisor/uuid or similar
        std::path::Path::new("/sys/class/dmi/id/product_uuid").exists()
            || std::path::Path::new("/sys/hypervisor/uuid").exists()
    }
}

/// AWS credential chain that tries multiple providers in order.
pub struct CredentialChain {
    providers: Vec<Box<dyn CredentialProvider>>,
}

impl std::fmt::Debug for CredentialChain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CredentialChain")
            .field("providers", &self.providers.len())
            .finish()
    }
}

impl Clone for CredentialChain {
    fn clone(&self) -> Self {
        // CredentialChain can't be cloned because of the trait objects
        // This is a limitation we accept
        Self::new()
    }
}

impl CredentialChain {
    /// Create a new credential chain with the default providers.
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    /// Add a provider to the chain.
    pub fn add_provider<P: CredentialProvider + 'static>(mut self, provider: P) -> Self {
        self.providers.push(Box::new(provider));
        self
    }

    /// Create the default credential chain.
    ///
    /// Resolution order:
    /// 1. Environment variables (AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY)
    /// 2. Shared credentials file (~/.aws/credentials)
    /// 3. Container credentials (ECS/EKS)
    /// 4. EC2 instance metadata service (IMDS)
    pub fn default_chain() -> Self {
        let chain = Self::new()
            .add_provider(EnvironmentCredentialProvider::new())
            .add_provider(ProfileCredentialProvider::default_profile())
            .add_provider(ContainerCredentialProvider::new())
            .add_provider(InstanceMetadataCredentialProvider::new());

        chain
    }

    /// Create a chain from environment and profile.
    pub fn from_environment() -> Self {
        Self::new()
            .add_provider(EnvironmentCredentialProvider::new())
            .add_provider(ProfileCredentialProvider::default_profile())
    }

    /// Resolve credentials by trying each provider in order.
    pub async fn resolve(&self) -> Result<AwsCredentials> {
        for provider in &self.providers {
            if !provider.is_valid() {
                continue;
            }

            match provider.provide_credentials().await {
                Ok(credentials) => {
                    return Ok(credentials);
                }
                Err(e) => {
                    trace!("Provider failed: {}", e);
                    continue;
                }
            }
        }

        Err(BedrockError::Credential {
            message: "No valid credentials found in chain".to_string(),
        })
    }
}

impl Default for CredentialChain {
    fn default() -> Self {
        Self::default_chain()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_aws_credentials() {
        let creds = AwsCredentials::new("AKIAIOSFODNN7EXAMPLE", "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY");
        assert_eq!(creds.access_key_id(), "AKIAIOSFODNN7EXAMPLE");
        assert!(!creds.is_temporary());
        assert!(!creds.is_expired());

        let temp_creds = AwsCredentials::with_session_token(
            "AKIAIOSFODNN7EXAMPLE",
            "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
            "FwoGZXIvYXdzEBYaDK...",
            Some(SystemTime::now() + Duration::from_secs(3600)),
        );
        assert!(temp_creds.is_temporary());
        assert!(!temp_creds.is_expired());
    }

    #[test]
    fn test_static_provider() {
        let creds = AwsCredentials::new("AKIAIOSFODNN7EXAMPLE", "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY");
        let provider = StaticCredentialProvider::new(creds.clone());

        // Can't test async here easily, but we can verify structure
        assert!(provider.is_valid());
    }

    #[test]
    fn test_environment_provider_validity() {
        let provider = EnvironmentCredentialProvider::new();

        // Depends on environment
        if env::var("AWS_ACCESS_KEY_ID").is_ok() {
            assert!(provider.is_valid());
        } else {
            assert!(!provider.is_valid());
        }
    }

    #[test]
    fn test_profile_provider_validity() {
        let provider = ProfileCredentialProvider::default_profile();
        // Depends on whether ~/.aws/credentials exists
        // Can't make assumptions in tests
    }
}
