//! Authentication middleware for the gateway.

use openrustclaw_core::error::{SecurityError, Error, Result};

/// Extract and validate the auth token from a request.
pub fn extract_token(auth_header: Option<&str>) -> Result<String> {
    let header = auth_header
        .ok_or(Error::Security(SecurityError::AuthRequired))?;

    let token = header
        .strip_prefix("Bearer ")
        .ok_or(Error::Security(SecurityError::TokenInvalid(
            "Missing Bearer prefix".to_string()
        )))?;

    Ok(token.to_string())
}
