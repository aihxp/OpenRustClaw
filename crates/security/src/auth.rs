//! Authentication and token management.

use chrono::{Utc, Duration};
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};
use serde::{Serialize, Deserialize};
use openrustclaw_core::error::{SecurityError, Error, Result};
use tracing::warn;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,  // user_id
    pub session_id: Option<String>,
    pub exp: i64,     // expiration timestamp
    pub iat: i64,     // issued at
}

/// Manages authentication tokens.
pub struct AuthManager {
    secret: String,
    token_duration_hours: i64,
}

impl AuthManager {
    pub fn new(secret: String) -> Self {
        Self {
            secret,
            token_duration_hours: 24,
        }
    }

    /// Generate a JWT token for a user.
    pub fn generate_token(&self, user_id: &str, session_id: Option<&str>) -> Result<String> {
        let now = Utc::now();
        let claims = Claims {
            sub: user_id.to_string(),
            session_id: session_id.map(String::from),
            exp: (now + Duration::hours(self.token_duration_hours)).timestamp(),
            iat: now.timestamp(),
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
        .map_err(|e| Error::Security(SecurityError::TokenInvalid(e.to_string())))
    }

    /// Validate a JWT token and return the claims.
    pub fn validate_token(&self, token: &str) -> Result<Claims> {
        decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &Validation::default(),
        )
        .map(|data| data.claims)
        .map_err(|e| {
            warn!(error = %e, "Token validation failed");
            match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                    Error::Security(SecurityError::TokenExpired)
                }
                _ => Error::Security(SecurityError::TokenInvalid(e.to_string())),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_and_validate_token() {
        let manager = AuthManager::new("test-secret-key-12345".to_string());
        let token = manager.generate_token("user_42", Some("session_1")).unwrap();
        let claims = manager.validate_token(&token).unwrap();
        assert_eq!(claims.sub, "user_42");
        assert_eq!(claims.session_id.as_deref(), Some("session_1"));
    }

    #[test]
    fn generate_token_without_session() {
        let manager = AuthManager::new("test-secret".to_string());
        let token = manager.generate_token("user_1", None).unwrap();
        let claims = manager.validate_token(&token).unwrap();
        assert_eq!(claims.sub, "user_1");
        assert!(claims.session_id.is_none());
    }

    #[test]
    fn reject_invalid_token() {
        let manager = AuthManager::new("test-secret".to_string());
        let result = manager.validate_token("invalid.jwt.token");
        assert!(result.is_err());
    }

    #[test]
    fn reject_wrong_secret() {
        let manager1 = AuthManager::new("secret-1".to_string());
        let manager2 = AuthManager::new("secret-2".to_string());
        let token = manager1.generate_token("user_1", None).unwrap();
        let result = manager2.validate_token(&token);
        assert!(result.is_err());
    }
}
