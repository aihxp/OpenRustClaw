//! Enterprise SSO (Single Sign-On) support
//!
//! Provides OIDC and SAML authentication for enterprise users.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod oidc;
pub mod saml;

pub use oidc::{OidcClient, OidcConfig};
pub use saml::{SamlClient, SamlConfig};

/// SSO provider types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SsoProvider {
    /// OpenID Connect
    Oidc,
    /// SAML 2.0
    Saml,
}

/// SSO client trait
#[async_trait]
pub trait SsoClient: Send + Sync {
    /// Initialize the SSO client
    async fn init(&mut self) -> Result<(), SsoError>;

    /// Generate authorization URL for login
    fn authorization_url(&self, state: &str, redirect_uri: &str) -> String;

    /// Exchange authorization code for tokens
    async fn exchange_code(&self, code: &str, redirect_uri: &str) -> Result<SsoTokens, SsoError>;

    /// Validate ID token and return user info
    async fn validate_token(&self, token: &str) -> Result<SsoUserInfo, SsoError>;

    /// Refresh access token
    async fn refresh_token(&self, refresh_token: &str) -> Result<SsoTokens, SsoError>;

    /// Logout user
    async fn logout(&self, token: &str) -> Result<(), SsoError>;

    /// Get provider metadata
    fn metadata(&self) -> &SsoMetadata;
}

/// SSO tokens returned after authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoTokens {
    /// Access token
    pub access_token: String,
    /// ID token (OIDC)
    pub id_token: Option<String>,
    /// Refresh token
    pub refresh_token: Option<String>,
    /// Token type (Bearer)
    pub token_type: String,
    /// Expiration time in seconds
    pub expires_in: i64,
    /// Scopes granted
    pub scope: Option<String>,
}

/// User information from SSO provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoUserInfo {
    /// Subject identifier (unique user ID)
    pub sub: String,
    /// Email address
    pub email: Option<String>,
    /// Email verified
    pub email_verified: Option<bool>,
    /// Full name
    pub name: Option<String>,
    /// Given name
    pub given_name: Option<String>,
    /// Family name
    pub family_name: Option<String>,
    /// Preferred username
    pub preferred_username: Option<String>,
    /// Groups/roles
    pub groups: Vec<String>,
    /// Organization
    pub organization: Option<String>,
    /// Department
    pub department: Option<String>,
    /// Raw claims from provider
    #[serde(flatten)]
    pub extra_claims: HashMap<String, serde_json::Value>,
}

/// SSO provider metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoMetadata {
    /// Provider type
    pub provider: SsoProvider,
    /// Issuer URL
    pub issuer: String,
    /// Authorization endpoint
    pub authorization_endpoint: String,
    /// Token endpoint
    pub token_endpoint: String,
    /// Userinfo endpoint (OIDC)
    pub userinfo_endpoint: Option<String>,
    /// JWKS endpoint (OIDC)
    pub jwks_uri: Option<String>,
    /// End session endpoint (OIDC)
    pub end_session_endpoint: Option<String>,
    /// Scopes supported
    pub scopes_supported: Vec<String>,
    /// Claims supported
    pub claims_supported: Vec<String>,
}

/// SSO configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoConfig {
    /// Enable SSO
    pub enabled: bool,
    /// Primary provider
    pub primary_provider: SsoProvider,
    /// Allowed domains for SSO
    pub allowed_domains: Vec<String>,
    /// Auto-provision users
    pub auto_provision: bool,
    /// Default role for new users
    pub default_role: String,
    /// OIDC configurations
    pub oidc: Vec<OidcConfig>,
    /// SAML configurations
    pub saml: Vec<SamlConfig>,
}

impl Default for SsoConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            primary_provider: SsoProvider::Oidc,
            allowed_domains: vec![],
            auto_provision: true,
            default_role: "user".to_string(),
            oidc: vec![],
            saml: vec![],
        }
    }
}

/// SSO errors
#[derive(Debug, thiserror::Error)]
pub enum SsoError {
    #[error("SSO is not enabled")]
    NotEnabled,

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Token validation failed: {0}")]
    TokenValidationFailed(String),

    #[error("User not found")]
    UserNotFound,

    #[error("Domain not allowed: {0}")]
    DomainNotAllowed(String),

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("SAML error: {0}")]
    SamlError(String),

    #[error("OIDC error: {0}")]
    OidcError(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// SSO registry for managing multiple providers
pub struct SsoRegistry {
    providers: HashMap<String, Box<dyn SsoClient>>,
    config: SsoConfig,
}

impl SsoRegistry {
    /// Create a new SSO registry
    pub fn new(config: SsoConfig) -> Self {
        Self {
            providers: HashMap::new(),
            config,
        }
    }

    /// Register a provider
    pub fn register(&mut self, name: String, provider: Box<dyn SsoClient>) {
        self.providers.insert(name, provider);
    }

    /// Get a provider by name
    pub fn get(&self, name: &str) -> Option<&dyn SsoClient> {
        self.providers.get(name).map(|p| p.as_ref())
    }

    /// Check if SSO is enabled
    pub fn enabled(&self) -> bool {
        self.config.enabled
    }

    /// Check if domain is allowed
    pub fn is_domain_allowed(&self, email: &str) -> bool {
        if self.config.allowed_domains.is_empty() {
            return true;
        }

        email
            .split('@')
            .nth(1)
            .map(|domain| self.config.allowed_domains.contains(&domain.to_string()))
            .unwrap_or(false)
    }

    /// Get default role
    pub fn default_role(&self) -> &str {
        &self.config.default_role
    }

    /// Should auto-provision users
    pub fn auto_provision(&self) -> bool {
        self.config.auto_provision
    }
}
