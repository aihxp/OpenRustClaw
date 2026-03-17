//! OpenID Connect (OIDC) SSO implementation

use super::{SsoClient, SsoError, SsoMetadata, SsoTokens, SsoUserInfo};
use async_trait::async_trait;
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// OIDC configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcConfig {
    /// Provider name (e.g., "okta", "auth0", "azure-ad")
    pub name: String,
    /// Client ID
    pub client_id: String,
    /// Client secret
    pub client_secret: String,
    /// Issuer URL
    pub issuer: String,
    /// Authorization endpoint (optional, discovered if not provided)
    pub authorization_endpoint: Option<String>,
    /// Token endpoint (optional, discovered if not provided)
    pub token_endpoint: Option<String>,
    /// Userinfo endpoint (optional, discovered if not provided)
    pub userinfo_endpoint: Option<String>,
    /// JWKS endpoint (optional, discovered if not provided)
    pub jwks_uri: Option<String>,
    /// End session endpoint (optional, discovered if not provided)
    pub end_session_endpoint: Option<String>,
    /// Scopes to request
    pub scopes: Vec<String>,
    /// Additional claims to request
    pub claims: Option<serde_json::Value>,
}

impl OidcConfig {
    /// Create a new OIDC configuration
    pub fn new(name: impl Into<String>, client_id: impl Into<String>, client_secret: impl Into<String>, issuer: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            issuer: issuer.into(),
            authorization_endpoint: None,
            token_endpoint: None,
            userinfo_endpoint: None,
            jwks_uri: None,
            end_session_endpoint: None,
            scopes: vec!["openid".to_string(), "email".to_string(), "profile".to_string()],
            claims: None,
        }
    }

    /// Add a scope
    pub fn with_scope(mut self, scope: impl Into<String>) -> Self {
        self.scopes.push(scope.into());
        self
    }
}

/// OIDC discovery document
#[derive(Debug, Clone, Deserialize)]
struct DiscoveryDocument {
    issuer: String,
    authorization_endpoint: String,
    token_endpoint: String,
    userinfo_endpoint: Option<String>,
    jwks_uri: Option<String>,
    end_session_endpoint: Option<String>,
    scopes_supported: Option<Vec<String>>,
    claims_supported: Option<Vec<String>>,
}

/// OIDC token response
#[derive(Debug, Clone, Deserialize)]
struct TokenResponse {
    access_token: String,
    id_token: Option<String>,
    refresh_token: Option<String>,
    token_type: String,
    expires_in: i64,
    scope: Option<String>,
}

/// OIDC client
pub struct OidcClient {
    config: OidcConfig,
    http_client: Client,
    metadata: SsoMetadata,
    jwks: Option<serde_json::Value>,
}

impl OidcClient {
    /// Create a new OIDC client
    pub fn new(config: OidcConfig) -> Self {
        let metadata = SsoMetadata {
            provider: super::SsoProvider::Oidc,
            issuer: config.issuer.clone(),
            authorization_endpoint: config.authorization_endpoint.clone().unwrap_or_default(),
            token_endpoint: config.token_endpoint.clone().unwrap_or_default(),
            userinfo_endpoint: config.userinfo_endpoint.clone(),
            jwks_uri: config.jwks_uri.clone(),
            end_session_endpoint: config.end_session_endpoint.clone(),
            scopes_supported: config.scopes.clone(),
            claims_supported: vec![],
        };

        Self {
            config,
            http_client: Client::new(),
            metadata,
            jwks: None,
        }
    }

    /// Fetch discovery document from issuer
    async fn discover(&mut self) -> Result<DiscoveryDocument, SsoError> {
        let discovery_url = format!("{}/.well-known/openid-configuration", self.config.issuer);
        
        let response = self.http_client
            .get(&discovery_url)
            .send()
            .await
            .map_err(|e| SsoError::OidcError(format!("Failed to fetch discovery: {}", e)))?;

        if !response.status().is_success() {
            return Err(SsoError::OidcError(
                format!("Discovery failed: {}", response.status())
            ));
        }

        let doc: DiscoveryDocument = response.json().await?;
        Ok(doc)
    }

    /// Fetch JWKS
    async fn fetch_jwks(&mut self) -> Result<(), SsoError> {
        let jwks_uri = self.metadata.jwks_uri.as_ref()
            .ok_or_else(|| SsoError::OidcError("JWKS URI not available".to_string()))?;

        let response = self.http_client
            .get(jwks_uri)
            .send()
            .await?;

        self.jwks = Some(response.json().await?);
        Ok(())
    }

    /// Verify ID token
    fn verify_id_token(&self, token: &str) -> Result<serde_json::Value, SsoError> {
        let header = decode_header(token)
            .map_err(|e| SsoError::TokenValidationFailed(format!("Invalid header: {}", e)))?;

        let kid = header.kid
            .ok_or_else(|| SsoError::TokenValidationFailed("No kid in header".to_string()))?;

        let jwks = self.jwks.as_ref()
            .ok_or_else(|| SsoError::TokenValidationFailed("JWKS not loaded".to_string()))?;

        // Find matching key
        let keys = jwks.get("keys")
            .and_then(|k| k.as_array())
            .ok_or_else(|| SsoError::TokenValidationFailed("Invalid JWKS format".to_string()))?;

        let key = keys.iter()
            .find(|k| k.get("kid").and_then(|k| k.as_str()) == Some(&kid))
            .ok_or_else(|| SsoError::TokenValidationFailed("Key not found in JWKS".to_string()))?;

        let n = key.get("n").and_then(|n| n.as_str())
            .ok_or_else(|| SsoError::TokenValidationFailed("Invalid key format".to_string()))?;
        let e = key.get("e").and_then(|e| e.as_str())
            .ok_or_else(|| SsoError::TokenValidationFailed("Invalid key format".to_string()))?;

        let decoding_key = DecodingKey::from_rsa_components(n, e)
            .map_err(|e| SsoError::TokenValidationFailed(format!("Invalid key: {}", e)))?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[&self.config.issuer]);
        validation.set_audience(&[&self.config.client_id]);

        let token_data = decode::<serde_json::Value>(token, &decoding_key, &validation)
            .map_err(|e| SsoError::TokenValidationFailed(format!("Token verification failed: {}", e)))?;

        Ok(token_data.claims)
    }
}

#[async_trait]
impl SsoClient for OidcClient {
    async fn init(&mut self) -> Result<(), SsoError> {
        // Discover endpoints if not provided
        if self.config.authorization_endpoint.is_none() {
            let discovery = self.discover().await?;
            
            self.metadata.authorization_endpoint = discovery.authorization_endpoint;
            self.metadata.token_endpoint = discovery.token_endpoint;
            self.metadata.userinfo_endpoint = discovery.userinfo_endpoint;
            self.metadata.jwks_uri = discovery.jwks_uri;
            self.metadata.end_session_endpoint = discovery.end_session_endpoint;
            
            if let Some(scopes) = discovery.scopes_supported {
                self.metadata.scopes_supported = scopes;
            }
            if let Some(claims) = discovery.claims_supported {
                self.metadata.claims_supported = claims;
            }
        }

        // Fetch JWKS
        self.fetch_jwks().await?;

        Ok(())
    }

    fn authorization_url(&self, state: &str, redirect_uri: &str) -> String {
        let mut url = format!(
            "{}?response_type=code&client_id={}&redirect_uri={}&state={}",
            self.metadata.authorization_endpoint,
            urlencoding::encode(&self.config.client_id),
            urlencoding::encode(redirect_uri),
            urlencoding::encode(state)
        );

        if !self.config.scopes.is_empty() {
            url.push_str(&format!("&scope={}", urlencoding::encode(&self.config.scopes.join(" "))));
        }

        if let Some(claims) = &self.config.claims {
            if let Ok(claims_json) = serde_json::to_string(claims) {
                url.push_str(&format!("&claims={}", urlencoding::encode(&claims_json)));
            }
        }

        url
    }

    async fn exchange_code(&self, code: &str, redirect_uri: &str) -> Result<SsoTokens, SsoError> {
        let mut params = HashMap::new();
        params.insert("grant_type", "authorization_code");
        params.insert("code", code);
        params.insert("redirect_uri", redirect_uri);
        params.insert("client_id", &self.config.client_id);
        params.insert("client_secret", &self.config.client_secret);

        let response = self.http_client
            .post(&self.metadata.token_endpoint)
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(SsoError::AuthenticationFailed(error_text));
        }

        let token_response: TokenResponse = response.json().await?;

        Ok(SsoTokens {
            access_token: token_response.access_token,
            id_token: token_response.id_token,
            refresh_token: token_response.refresh_token,
            token_type: token_response.token_type,
            expires_in: token_response.expires_in,
            scope: token_response.scope,
        })
    }

    async fn validate_token(&self, token: &str) -> Result<SsoUserInfo, SsoError> {
        // Try to verify ID token first
        let claims = if let Ok(claims) = self.verify_id_token(token) {
            claims
        } else {
            // Fall back to userinfo endpoint
            let userinfo_url = self.metadata.userinfo_endpoint.as_ref()
                .ok_or_else(|| SsoError::OidcError("No userinfo endpoint".to_string()))?;

            let response = self.http_client
                .get(userinfo_url)
                .bearer_auth(token)
                .send()
                .await?;

            if !response.status().is_success() {
                return Err(SsoError::TokenValidationFailed("Userinfo request failed".to_string()));
            }

            response.json().await?
        };

        let groups = claims.get("groups")
            .and_then(|g| g.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        Ok(SsoUserInfo {
            sub: claims.get("sub").and_then(|s| s.as_str()).unwrap_or("").to_string(),
            email: claims.get("email").and_then(|e| e.as_str()).map(String::from),
            email_verified: claims.get("email_verified").and_then(|v| v.as_bool()),
            name: claims.get("name").and_then(|n| n.as_str()).map(String::from),
            given_name: claims.get("given_name").and_then(|n| n.as_str()).map(String::from),
            family_name: claims.get("family_name").and_then(|n| n.as_str()).map(String::from),
            preferred_username: claims.get("preferred_username").and_then(|u| u.as_str()).map(String::from),
            groups,
            organization: claims.get("org_name").or_else(|| claims.get("organization")).and_then(|o| o.as_str()).map(String::from),
            department: claims.get("department").and_then(|d| d.as_str()).map(String::from),
            extra_claims: claims.as_object().map(|o| o.iter().map(|(k, v)| (k.clone(), v.clone())).collect()).unwrap_or_default(),
        })
    }

    async fn refresh_token(&self, refresh_token: &str) -> Result<SsoTokens, SsoError> {
        let mut params = HashMap::new();
        params.insert("grant_type", "refresh_token");
        params.insert("refresh_token", refresh_token);
        params.insert("client_id", &self.config.client_id);
        params.insert("client_secret", &self.config.client_secret);

        let response = self.http_client
            .post(&self.metadata.token_endpoint)
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(SsoError::AuthenticationFailed(error_text));
        }

        let token_response: TokenResponse = response.json().await?;

        Ok(SsoTokens {
            access_token: token_response.access_token,
            id_token: token_response.id_token,
            refresh_token: token_response.refresh_token,
            token_type: token_response.token_type,
            expires_in: token_response.expires_in,
            scope: token_response.scope,
        })
    }

    async fn logout(&self, _token: &str) -> Result<(), SsoError> {
        // OIDC RP-initiated logout
        // In practice, this would redirect to the end_session_endpoint
        Ok(())
    }

    fn metadata(&self) -> &SsoMetadata {
        &self.metadata
    }
}
