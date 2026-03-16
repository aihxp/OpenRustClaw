//! WebSocket origin validation.
//!
//! Fixes CVE-2026-25253: OpenClaw trusted localhost WebSocket connections
//! without authentication. OpenRustClaw requires mandatory origin validation
//! on ALL connections, including localhost.

use openrustclaw_core::error::{SecurityError, Error, Result};
use tracing::warn;

/// Validates WebSocket connection origins against an allow-list.
pub struct OriginValidator {
    allowed_origins: Vec<String>,
}

impl OriginValidator {
    pub fn new(allowed_origins: Vec<String>) -> Self {
        Self { allowed_origins }
    }

    /// Validate an origin header value. Returns Ok(()) if allowed, Err if rejected.
    pub fn validate(&self, origin: &str) -> Result<()> {
        // Parse and normalize the origin
        let normalized = self.normalize_origin(origin);

        if self.allowed_origins.iter().any(|allowed| {
            self.normalize_origin(allowed) == normalized
        }) {
            Ok(())
        } else {
            warn!(origin = %origin, "Rejected WebSocket connection from unauthorized origin");
            Err(Error::Security(SecurityError::InvalidOrigin {
                origin: origin.to_string(),
            }))
        }
    }

    fn normalize_origin(&self, origin: &str) -> String {
        // Strip trailing slashes and lowercase
        origin.trim_end_matches('/').to_lowercase()
    }

    /// Check if any origins are configured.
    pub fn has_origins(&self) -> bool {
        !self.allowed_origins.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_allowed_origin() {
        let validator = OriginValidator::new(vec!["https://example.com".to_string()]);
        assert!(validator.validate("https://example.com").is_ok());
    }

    #[test]
    fn rejects_unknown_origin() {
        let validator = OriginValidator::new(vec!["https://example.com".to_string()]);
        assert!(validator.validate("https://evil.com").is_err());
    }

    #[test]
    fn normalizes_trailing_slash() {
        let validator = OriginValidator::new(vec!["https://example.com".to_string()]);
        assert!(validator.validate("https://example.com/").is_ok());
    }

    #[test]
    fn normalizes_case() {
        let validator = OriginValidator::new(vec!["https://Example.COM".to_string()]);
        assert!(validator.validate("https://example.com").is_ok());
    }

    #[test]
    fn empty_origins_rejects_all() {
        let validator = OriginValidator::new(vec![]);
        assert!(!validator.has_origins());
        assert!(validator.validate("https://example.com").is_err());
    }

    #[test]
    fn localhost_not_auto_trusted() {
        let validator = OriginValidator::new(vec!["https://example.com".to_string()]);
        assert!(validator.validate("http://localhost:3000").is_err());
    }

    #[test]
    fn localhost_allowed_when_configured() {
        let validator = OriginValidator::new(vec![
            "https://example.com".to_string(),
            "http://localhost:3000".to_string(),
        ]);
        assert!(validator.validate("http://localhost:3000").is_ok());
    }
}
