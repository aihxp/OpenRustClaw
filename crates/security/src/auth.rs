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

    // ── Error path tests ──

    #[test]
    fn reject_expired_token() {
        // Create a manager with a very short (or already-expired) token duration.
        // We'll manually craft an expired JWT to avoid needing to sleep.
        let secret = "test-secret-for-expiry";
        let manager = AuthManager::new(secret.to_string());

        // Build an expired claims set directly
        let now = Utc::now();
        let expired_claims = Claims {
            sub: "user_expired".to_string(),
            session_id: None,
            exp: (now - Duration::hours(1)).timestamp(), // expired 1 hour ago
            iat: (now - Duration::hours(2)).timestamp(),
        };

        let token = jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            &expired_claims,
            &jsonwebtoken::EncodingKey::from_secret(secret.as_bytes()),
        )
        .expect("encoding should not fail");

        let result = manager.validate_token(&token);
        assert!(result.is_err(), "Expired token should be rejected");
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("expired") || err_msg.contains("Token"),
            "Error should mention expiration: {}",
            err_msg
        );
    }

    #[test]
    fn reject_empty_token() {
        let manager = AuthManager::new("test-secret".to_string());
        let result = manager.validate_token("");
        assert!(result.is_err(), "Empty token should be rejected");
    }

    #[test]
    fn reject_malformed_token_random_string() {
        let manager = AuthManager::new("test-secret".to_string());
        let result = manager.validate_token("not-a-jwt-at-all");
        assert!(result.is_err(), "Random string should be rejected");
    }

    #[test]
    fn reject_malformed_token_partial_jwt() {
        let manager = AuthManager::new("test-secret".to_string());
        // A JWT has 3 parts separated by dots; provide only 2
        let result = manager.validate_token("eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ0ZXN0In0");
        assert!(result.is_err(), "Partial JWT (2 parts) should be rejected");
    }

    #[test]
    fn reject_malformed_token_tampered_payload() {
        let manager = AuthManager::new("test-secret".to_string());
        // Generate a valid token, then tamper with the payload
        let valid_token = manager.generate_token("user_1", None).unwrap();
        let parts: Vec<&str> = valid_token.split('.').collect();
        assert_eq!(parts.len(), 3, "Valid JWT should have 3 parts");

        // Replace the payload with garbage but keep header and signature
        let tampered = format!("{}.dGFtcGVyZWQ.{}", parts[0], parts[2]);
        let result = manager.validate_token(&tampered);
        assert!(result.is_err(), "Tampered JWT should be rejected");
    }

    #[test]
    fn reject_token_with_wrong_algorithm_claim() {
        let manager = AuthManager::new("test-secret".to_string());
        // This is a completely fabricated token with none algorithm
        let result = manager.validate_token(
            "eyJhbGciOiJub25lIiwidHlwIjoiSldUIn0.eyJzdWIiOiJ0ZXN0IiwiZXhwIjo5OTk5OTk5OTk5fQ."
        );
        assert!(result.is_err(), "Token with 'none' algorithm should be rejected");
    }
}
