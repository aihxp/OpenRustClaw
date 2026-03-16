//! OAuth and authentication management
//!
//! This module provides OAuth 2.0 support with PKCE for MCP servers
//! and OpenAPI specs that require authentication.

use crate::error::{Mcp2CliError, Result};
use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};

use tracing::{info, warn};

/// OAuth configuration for a server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    /// OAuth authorization endpoint
    pub auth_url: String,
    /// OAuth token endpoint
    pub token_url: String,
    /// Client ID
    pub client_id: String,
    /// Client secret (optional for PKCE)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<String>,
    /// Redirect URI
    pub redirect_uri: String,
    /// Scopes to request
    pub scopes: Vec<String>,
    /// Use PKCE (recommended)
    #[serde(default = "default_true")]
    pub use_pkce: bool,
}

fn default_true() -> bool {
    true
}

impl OAuthConfig {
    /// Create a new OAuth configuration
    pub fn new(
        auth_url: impl Into<String>,
        token_url: impl Into<String>,
        client_id: impl Into<String>,
        redirect_uri: impl Into<String>,
    ) -> Self {
        Self {
            auth_url: auth_url.into(),
            token_url: token_url.into(),
            client_id: client_id.into(),
            client_secret: None,
            redirect_uri: redirect_uri.into(),
            scopes: Vec::new(),
            use_pkce: true,
        }
    }

    /// Set client secret
    pub fn with_client_secret(mut self, secret: impl Into<String>) -> Self {
        self.client_secret = Some(secret.into());
        self
    }

    /// Add a scope
    pub fn with_scope(mut self, scope: impl Into<String>) -> Self {
        self.scopes.push(scope.into());
        self
    }

    /// Set scopes
    pub fn with_scopes(mut self, scopes: Vec<String>) -> Self {
        self.scopes = scopes;
        self
    }

    /// Disable PKCE
    pub fn without_pkce(mut self) -> Self {
        self.use_pkce = false;
        self
    }

    /// Build the authorization URL
    pub fn build_auth_url(&self, state: &str, code_challenge: Option<&str>) -> String {
        let mut url = format!(
            "{}?response_type=code&client_id={}&redirect_uri={}&state={}",
            self.auth_url,
            urlencoding::encode(&self.client_id),
            urlencoding::encode(&self.redirect_uri),
            urlencoding::encode(state)
        );

        if !self.scopes.is_empty() {
            let scope_str = self.scopes.join(" ");
            url.push_str(&format!("&scope={}", urlencoding::encode(&scope_str)));
        }

        if let Some(challenge) = code_challenge {
            url.push_str(&format!(
                "&code_challenge={}&code_challenge_method=S256",
                challenge
            ));
        }

        url
    }
}

/// OAuth token response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    /// Access token
    pub access_token: String,
    /// Token type (usually "Bearer")
    pub token_type: String,
    /// Refresh token (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    /// Expiration time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    /// Scopes granted
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

impl Token {
    /// Check if the token is expired (or about to expire)
    pub fn is_expired(&self) -> bool {
        match self.expires_at {
            None => false, // No expiration = never expires
            Some(expires) => Utc::now() + Duration::minutes(5) > expires,
        }
    }

    /// Get the Authorization header value
    pub fn auth_header(&self) -> String {
        format!("{} {}", self.token_type, self.access_token)
    }
}

/// PKCE code verifier and challenge
#[derive(Debug, Clone)]
pub struct PkcePair {
    /// Code verifier (sent in token request)
    pub verifier: String,
    /// Code challenge (sent in auth request)
    pub challenge: String,
}

impl PkcePair {
    /// Generate a new PKCE pair
    pub fn generate() -> Self {
        let verifier = generate_code_verifier();
        let challenge = generate_code_challenge(&verifier);
        Self { verifier, challenge }
    }
}

/// Authentication manager
pub struct AuthManager {
    /// Stored tokens keyed by server URL
    tokens: DashMap<String, Token>,
    /// OAuth configurations keyed by server URL
    configs: DashMap<String, OAuthConfig>,
    /// Pending OAuth flows (state -> PKCE pair)
    pending_flows: DashMap<String, (String, PkcePair)>, // state -> (server_url, pkce)
}

impl AuthManager {
    /// Create a new authentication manager
    pub fn new() -> Self {
        Self {
            tokens: DashMap::new(),
            configs: DashMap::new(),
            pending_flows: DashMap::new(),
        }
    }

    /// Register an OAuth configuration for a server
    pub fn register_config(&self, server_url: impl Into<String>, config: OAuthConfig) {
        self.configs.insert(server_url.into(), config);
    }

    /// Start an OAuth flow
    ///
    /// Returns the authorization URL to redirect the user to
    pub fn start_oauth_flow(&self, server_url: &str) -> Result<String> {
        let config = self
            .configs
            .get(server_url)
            .ok_or_else(|| Mcp2CliError::auth(format!("No OAuth config for server: {}", server_url)))?;

        let state = generate_state();
        let pkce = if config.use_pkce {
            Some(PkcePair::generate())
        } else {
            None
        };

        let auth_url = config.build_auth_url(
            &state,
            pkce.as_ref().map(|p| p.challenge.as_str()),
        );

        // Store pending flow
        self.pending_flows.insert(
            state.clone(),
            (server_url.to_string(), pkce.unwrap_or_else(|| PkcePair { verifier: String::new(), challenge: String::new() })),
        );

        info!(server_url = %server_url, "Started OAuth flow");

        Ok(auth_url)
    }

    /// Complete an OAuth flow with the authorization code
    pub async fn complete_oauth_flow(&self, state: &str, code: &str) -> Result<Token> {
        let (server_url, pkce) = self
            .pending_flows
            .remove(state)
            .ok_or_else(|| Mcp2CliError::auth("Invalid or expired state parameter"))?
            .1;

        let config = self
            .configs
            .get(&server_url)
            .ok_or_else(|| Mcp2CliError::auth("OAuth config not found"))?;

        // Exchange code for token
        let token = self.exchange_code(&config, code, &pkce.verifier).await?;

        // Store the token
        self.tokens.insert(server_url.clone(), token.clone());

        info!(server_url = %server_url, "Completed OAuth flow successfully");

        Ok(token)
    }

    /// Exchange authorization code for token
    async fn exchange_code(&self, config: &OAuthConfig, code: &str, code_verifier: &str) -> Result<Token> {
        let client = reqwest::Client::new();

        let mut params = vec![
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", &config.redirect_uri),
            ("client_id", &config.client_id),
        ];

        if let Some(ref secret) = config.client_secret {
            params.push(("client_secret", secret));
        }

        if config.use_pkce && !code_verifier.is_empty() {
            params.push(("code_verifier", code_verifier));
        }

        let response = client
            .post(&config.token_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| Mcp2CliError::Http(e))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Mcp2CliError::auth(format!(
                "Token exchange failed: {}",
                error_text
            )));
        }

        let mut token_response: serde_json::Value = response
            .json()
            .await
            .map_err(|e| Mcp2CliError::Http(e))?;

        // Calculate expires_at from expires_in
        if let Some(expires_in) = token_response.get("expires_in").and_then(|v| v.as_i64()) {
            let expires_at = Utc::now() + Duration::seconds(expires_in);
            token_response["expires_at"] = serde_json::json!(expires_at);
        }

        let token: Token = serde_json::from_value(token_response)
            .map_err(|e| Mcp2CliError::Json(e))?;

        Ok(token)
    }

    /// Get a stored token for a server
    pub fn get_token(&self, server_url: &str) -> Option<Token> {
        self.tokens.get(server_url).map(|t| t.clone())
    }

    /// Get a valid (non-expired) token, refreshing if necessary
    pub async fn get_valid_token(&self, server_url: &str) -> Option<Token> {
        let token = self.get_token(server_url)?;

        if token.is_expired() {
            // Try to refresh
            if let Some(refresh_token) = &token.refresh_token {
                match self.refresh_token(server_url, refresh_token).await {
                    Ok(new_token) => return Some(new_token),
                    Err(e) => {
                        warn!(error = %e, "Failed to refresh token");
                        return None;
                    }
                }
            }
            None
        } else {
            Some(token)
        }
    }

    /// Refresh an access token
    async fn refresh_token(&self, server_url: &str, refresh_token: &str) -> Result<Token> {
        let config = self
            .configs
            .get(server_url)
            .ok_or_else(|| Mcp2CliError::auth("OAuth config not found"))?;

        let client = reqwest::Client::new();

        let mut params = vec![
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", &config.client_id),
        ];

        if let Some(ref secret) = config.client_secret {
            params.push(("client_secret", secret));
        }

        let response = client
            .post(&config.token_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| Mcp2CliError::Http(e))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Mcp2CliError::auth(format!(
                "Token refresh failed: {}",
                error_text
            )));
        }

        let mut token_response: serde_json::Value = response
            .json()
            .await
            .map_err(|e| Mcp2CliError::Http(e))?;

        // Calculate expires_at from expires_in
        if let Some(expires_in) = token_response.get("expires_in").and_then(|v| v.as_i64()) {
            let expires_at = Utc::now() + Duration::seconds(expires_in);
            token_response["expires_at"] = serde_json::json!(expires_at);
        }

        let token: Token = serde_json::from_value(token_response)
            .map_err(|e| Mcp2CliError::Json(e))?;

        // Store the new token
        self.tokens.insert(server_url.to_string(), token.clone());

        Ok(token)
    }

    /// Set a token directly (e.g., from API key or pre-obtained token)
    pub fn set_token(&self, server_url: impl Into<String>, token: Token) {
        self.tokens.insert(server_url.into(), token);
    }

    /// Set a bearer token directly
    pub fn set_bearer_token(&self, server_url: impl Into<String>, access_token: impl Into<String>) {
        let token = Token {
            access_token: access_token.into(),
            token_type: "Bearer".to_string(),
            refresh_token: None,
            expires_at: None,
            scope: None,
        };
        self.tokens.insert(server_url.into(), token);
    }

    /// Clear all tokens
    pub fn clear_tokens(&self) {
        self.tokens.clear();
    }

    /// Clear token for a specific server
    pub fn clear_token(&self, server_url: &str) {
        self.tokens.remove(server_url);
    }

    /// Check if we have a token for a server
    pub fn has_token(&self, server_url: &str) -> bool {
        self.tokens.contains_key(server_url)
    }

    /// Get all server URLs with tokens
    pub fn list_servers(&self) -> Vec<String> {
        self.tokens.iter().map(|e| e.key().clone()).collect()
    }
}

impl Default for AuthManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate a random state parameter
fn generate_state() -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect()
}

/// Generate PKCE code verifier (43-128 chars)
fn generate_code_verifier() -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(128)
        .map(char::from)
        .collect()
}

/// Generate PKCE code challenge (SHA256 hash of verifier, base64url encoded)
fn generate_code_challenge(verifier: &str) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let hash = hasher.finalize();

    // Base64url encode without padding
    base64_url_encode(&hash)
}

/// Base64url encoding (URL-safe base64 without padding)
fn base64_url_encode(input: &[u8]) -> String {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    URL_SAFE_NO_PAD.encode(input)
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oauth_config() {
        let config = OAuthConfig::new(
            "https://auth.example.com/authorize",
            "https://auth.example.com/token",
            "client123",
            "http://localhost:8080/callback",
        )
        .with_scope("read")
        .with_scope("write");

        let state = "test_state";
        let pkce = PkcePair::generate();
        let auth_url = config.build_auth_url(state, Some(&pkce.challenge));

        assert!(auth_url.contains("response_type=code"));
        assert!(auth_url.contains("client_id=client123"));
        assert!(auth_url.contains("state=test_state"));
        assert!(auth_url.contains("code_challenge="));
        assert!(auth_url.contains("code_challenge_method=S256"));
    }

    #[test]
    fn test_pkce_generation() {
        let pkce = PkcePair::generate();
        
        assert!(!pkce.verifier.is_empty());
        assert!(!pkce.challenge.is_empty());
        assert_ne!(pkce.verifier, pkce.challenge);
        
        // Verifier should be reproducible
        let challenge2 = generate_code_challenge(&pkce.verifier);
        assert_eq!(pkce.challenge, challenge2);
    }

    #[test]
    fn test_token_expiration() {
        let token = Token {
            access_token: "test".to_string(),
            token_type: "Bearer".to_string(),
            refresh_token: None,
            expires_at: Some(Utc::now() + Duration::hours(1)),
            scope: None,
        };
        
        assert!(!token.is_expired());
        
        let expired_token = Token {
            access_token: "test".to_string(),
            token_type: "Bearer".to_string(),
            refresh_token: None,
            expires_at: Some(Utc::now() - Duration::hours(1)),
            scope: None,
        };
        
        assert!(expired_token.is_expired());
    }

    #[test]
    fn test_auth_manager() {
        let manager = AuthManager::new();
        
        // Set a token
        manager.set_bearer_token("https://api.example.com", "token123");
        
        assert!(manager.has_token("https://api.example.com"));
        
        let token = manager.get_token("https://api.example.com").unwrap();
        assert_eq!(token.access_token, "token123");
        assert_eq!(token.auth_header(), "Bearer token123");
        
        // Clear token
        manager.clear_token("https://api.example.com");
        assert!(!manager.has_token("https://api.example.com"));
    }

    #[test]
    fn test_base64_url() {
        let data = b"hello world";
        let encoded = base64_url_encode(data);
        // Test encoding is correct by decoding with standard base64
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
        let decoded = URL_SAFE_NO_PAD.decode(&encoded).unwrap();
        assert_eq!(data.to_vec(), decoded);
    }
}
