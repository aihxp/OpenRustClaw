//! Error types for the AWS Bedrock SDK.

use std::fmt;

/// Result type alias for Bedrock operations.
pub type Result<T> = std::result::Result<T, BedrockError>;

/// Errors that can occur when using AWS Bedrock.
#[derive(Debug)]
pub enum BedrockError {
    /// Authentication failed (invalid credentials).
    Authentication {
        /// Error message
        message: String,
    },

    /// Access denied (insufficient permissions).
    AccessDenied {
        /// Error message
        message: String,
    },

    /// Rate limit exceeded.
    RateLimit {
        /// Retry after duration, if known
        retry_after: Option<std::time::Duration>,
        /// Error message
        message: String,
    },

    /// Throttling error (too many requests).
    Throttling {
        /// Retry after duration, if known
        retry_after: Option<std::time::Duration>,
        /// Error message
        message: String,
    },

    /// Context length exceeded.
    ContextLength {
        /// Tokens used
        tokens_used: usize,
        /// Maximum tokens allowed
        max_tokens: usize,
        /// Error message
        message: String,
    },

    /// Invalid request (bad parameters).
    InvalidRequest {
        /// Error message
        message: String,
    },

    /// Model not found or not available.
    ModelNotFound {
        /// Model identifier
        model: String,
        /// Error message
        message: String,
    },

    /// Model error (e.g., model stopped generating).
    ModelError {
        /// Error message
        message: String,
    },

    /// Validation error (request validation failed).
    Validation {
        /// Field that failed validation
        field: Option<String>,
        /// Error message
        message: String,
    },

    /// Service quota exceeded.
    ServiceQuotaExceeded {
        /// Quota code
        quota_code: Option<String>,
        /// Error message
        message: String,
    },

    /// API error from Bedrock.
    Api {
        /// HTTP status code
        status: reqwest::StatusCode,
        /// Error code from AWS
        error_code: Option<String>,
        /// Error message
        message: String,
        /// Request ID for debugging
        request_id: Option<String>,
    },

    /// Network/HTTP error.
    Http {
        /// Source error
        source: reqwest::Error,
    },

    /// JSON serialization/deserialization error.
    Json {
        /// Source error
        source: serde_json::Error,
    },

    /// AWS SigV4 signing error.
    SigV4 {
        /// Error message
        message: String,
    },

    /// Credential error.
    Credential {
        /// Error message
        message: String,
    },

    /// Stream processing error.
    Stream {
        /// Error message
        message: String,
    },

    /// Configuration error.
    Config {
        /// Error message
        message: String,
    },

    /// Timeout error.
    Timeout {
        /// Operation that timed out
        operation: String,
    },

    /// Retry exhaustion.
    RetryExhausted {
        /// Number of attempts made
        attempts: u32,
        /// Last error
        last_error: Box<BedrockError>,
    },

    /// Guardrail intervention (content blocked).
    GuardrailIntervention {
        /// Guardrail ID
        guardrail_id: String,
        /// Guardrail version
        guardrail_version: String,
        /// Action taken
        action: String,
        /// Error message
        message: String,
    },

    /// Internal error.
    Internal {
        /// Error message
        message: String,
    },

    /// Not implemented.
    NotImplemented {
        /// Feature name
        feature: String,
    },
}

impl fmt::Display for BedrockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BedrockError::Authentication { message } => {
                write!(f, "Authentication failed: {message}")
            }
            BedrockError::AccessDenied { message } => {
                write!(f, "Access denied: {message}")
            }
            BedrockError::RateLimit { retry_after, message } => {
                if let Some(duration) = retry_after {
                    write!(f, "Rate limit exceeded (retry after {duration:?}): {message}")
                } else {
                    write!(f, "Rate limit exceeded: {message}")
                }
            }
            BedrockError::Throttling { retry_after, message } => {
                if let Some(duration) = retry_after {
                    write!(f, "Throttling (retry after {duration:?}): {message}")
                } else {
                    write!(f, "Throttling: {message}")
                }
            }
            BedrockError::ContextLength {
                tokens_used,
                max_tokens,
                message,
            } => {
                write!(
                    f,
                    "Context length exceeded: {tokens_used} tokens used, {max_tokens} max: {message}"
                )
            }
            BedrockError::InvalidRequest { message } => {
                write!(f, "Invalid request: {message}")
            }
            BedrockError::ModelNotFound { model, message } => {
                write!(f, "Model not found ({model}): {message}")
            }
            BedrockError::ModelError { message } => {
                write!(f, "Model error: {message}")
            }
            BedrockError::Validation { field, message } => {
                if let Some(field) = field {
                    write!(f, "Validation error ({field}): {message}")
                } else {
                    write!(f, "Validation error: {message}")
                }
            }
            BedrockError::ServiceQuotaExceeded { quota_code, message } => {
                if let Some(code) = quota_code {
                    write!(f, "Service quota exceeded ({code}): {message}")
                } else {
                    write!(f, "Service quota exceeded: {message}")
                }
            }
            BedrockError::Api {
                status,
                error_code,
                message,
                request_id,
            } => {
                if let Some(code) = error_code {
                    write!(f, "API error ({status} - {code}): {message}")?;
                } else {
                    write!(f, "API error ({status}): {message}")?;
                }
                if let Some(req_id) = request_id {
                    write!(f, " (Request ID: {req_id})")?;
                }
                Ok(())
            }
            BedrockError::Http { source } => {
                write!(f, "HTTP error: {source}")
            }
            BedrockError::Json { source } => {
                write!(f, "JSON error: {source}")
            }
            BedrockError::SigV4 { message } => {
                write!(f, "SigV4 signing error: {message}")
            }
            BedrockError::Credential { message } => {
                write!(f, "Credential error: {message}")
            }
            BedrockError::Stream { message } => {
                write!(f, "Stream error: {message}")
            }
            BedrockError::Config { message } => {
                write!(f, "Configuration error: {message}")
            }
            BedrockError::Timeout { operation } => {
                write!(f, "Timeout during {operation}")
            }
            BedrockError::RetryExhausted { attempts, last_error } => {
                write!(f, "Retry exhausted after {attempts} attempts: {last_error}")
            }
            BedrockError::GuardrailIntervention {
                guardrail_id,
                guardrail_version,
                action,
                message,
            } => {
                write!(
                    f,
                    "Guardrail intervention ({guardrail_id}/{guardrail_version} - {action}): {message}"
                )
            }
            BedrockError::Internal { message } => {
                write!(f, "Internal error: {message}")
            }
            BedrockError::NotImplemented { feature } => {
                write!(f, "Feature not implemented: {feature}")
            }
        }
    }
}

impl std::error::Error for BedrockError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            BedrockError::Http { source } => Some(source),
            BedrockError::Json { source } => Some(source),
            BedrockError::RetryExhausted { last_error, .. } => Some(last_error),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for BedrockError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            BedrockError::Timeout {
                operation: "HTTP request".to_string(),
            }
        } else if err.is_connect() {
            BedrockError::Http { source: err }
        } else {
            BedrockError::Http { source: err }
        }
    }
}

impl From<serde_json::Error> for BedrockError {
    fn from(err: serde_json::Error) -> Self {
        BedrockError::Json { source: err }
    }
}

impl From<std::io::Error> for BedrockError {
    fn from(err: std::io::Error) -> Self {
        BedrockError::Internal {
            message: format!("IO error: {err}"),
        }
    }
}

impl From<aws_sigv4::http_request::SigningError> for BedrockError {
    fn from(err: aws_sigv4::http_request::SigningError) -> Self {
        BedrockError::SigV4 {
            message: err.to_string(),
        }
    }
}

impl BedrockError {
    /// Check if this error is retryable.
    pub fn is_retryable(&self) -> bool {
        match self {
            BedrockError::RateLimit { .. }
            | BedrockError::Throttling { .. }
            | BedrockError::Http { .. } => true,
            BedrockError::Api { status, .. } => {
                status.is_server_error()
                    || *status == reqwest::StatusCode::TOO_MANY_REQUESTS
                    || *status == reqwest::StatusCode::REQUEST_TIMEOUT
            }
            _ => false,
        }
    }

    /// Check if this is a rate limit error.
    pub fn is_rate_limit(&self) -> bool {
        matches!(
            self,
            BedrockError::RateLimit { .. } | BedrockError::Throttling { .. }
        )
    }

    /// Check if this is an authentication error.
    pub fn is_auth_error(&self) -> bool {
        matches!(
            self,
            BedrockError::Authentication { .. } | BedrockError::Credential { .. }
        )
    }

    /// Check if this is an access denied error.
    pub fn is_access_denied(&self) -> bool {
        matches!(self, BedrockError::AccessDenied { .. })
    }

    /// Get the retry-after duration if available.
    pub fn retry_after(&self) -> Option<std::time::Duration> {
        match self {
            BedrockError::RateLimit { retry_after, .. } => *retry_after,
            BedrockError::Throttling { retry_after, .. } => *retry_after,
            _ => None,
        }
    }

    /// Create an error from an HTTP response.
    pub(crate) async fn from_response(response: reqwest::Response) -> Self {
        let status = response.status();
        let request_id = response
            .headers()
            .get("x-amzn-requestid")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let body = response.text().await.unwrap_or_default();

        // Try to parse AWS error format
        if let Ok(error_json) = serde_json::from_str::<serde_json::Value>(&body) {
            if let Some(error_type) = error_json.get("__type").and_then(|v| v.as_str()) {
                let message = error_json
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&body)
                    .to_string();

                return Self::from_aws_error(error_type, &message, status, request_id);
            }

            // Alternative error format
            if let Some(message) = error_json.get("Message").and_then(|v| v.as_str()) {
                let error_type = error_json
                    .get("Code")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");
                return Self::from_aws_error(error_type, message, status, request_id);
            }
        }

        // Fallback for non-JSON responses
        BedrockError::Api {
            status,
            error_code: None,
            message: body,
            request_id,
        }
    }

    /// Parse AWS error type into specific error variant.
    fn from_aws_error(
        error_type: &str,
        message: &str,
        status: reqwest::StatusCode,
        request_id: Option<String>,
    ) -> Self {
        match error_type {
            "AccessDeniedException" | "AccessDenied" => BedrockError::AccessDenied {
                message: message.to_string(),
            },
            "UnrecognizedClientException" | "InvalidSignatureException" => {
                BedrockError::Authentication {
                    message: message.to_string(),
                }
            }
            "ThrottlingException" => BedrockError::Throttling {
                retry_after: None,
                message: message.to_string(),
            },
            "ServiceQuotaExceededException" => BedrockError::ServiceQuotaExceeded {
                quota_code: None,
                message: message.to_string(),
            },
            "ValidationException" => BedrockError::Validation {
                field: None,
                message: message.to_string(),
            },
            "ResourceNotFoundException" => BedrockError::ModelNotFound {
                model: "unknown".to_string(),
                message: message.to_string(),
            },
            "ModelErrorException" | "ModelTimeoutException" => BedrockError::ModelError {
                message: message.to_string(),
            },
            "ModelNotReadyException" => BedrockError::ModelNotFound {
                model: "unknown".to_string(),
                message: message.to_string(),
            },
            "InternalServerException" => BedrockError::Internal {
                message: message.to_string(),
            },
            "ModelStreamErrorException" => BedrockError::Stream {
                message: message.to_string(),
            },
            "ConflictException" => BedrockError::InvalidRequest {
                message: message.to_string(),
            },
            "ModelNotAvailableException" => BedrockError::ModelNotFound {
                model: "unknown".to_string(),
                message: message.to_string(),
            },
            _ => BedrockError::Api {
                status,
                error_code: Some(error_type.to_string()),
                message: message.to_string(),
                request_id,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = BedrockError::Authentication {
            message: "Invalid credentials".to_string(),
        };
        assert!(err.to_string().contains("Authentication failed"));
    }

    #[test]
    fn test_is_retryable() {
        let rate_limit = BedrockError::RateLimit {
            retry_after: None,
            message: "Too many requests".to_string(),
        };
        assert!(rate_limit.is_retryable());

        let auth = BedrockError::Authentication {
            message: "Invalid credentials".to_string(),
        };
        assert!(!auth.is_retryable());

        let validation = BedrockError::Validation {
            field: Some("modelId".to_string()),
            message: "Invalid model".to_string(),
        };
        assert!(!validation.is_retryable());
    }

    #[test]
    fn test_aws_error_parsing() {
        // Test AccessDenied
        let err = BedrockError::from_aws_error(
            "AccessDeniedException",
            "Access denied",
            reqwest::StatusCode::FORBIDDEN,
            Some("req-123".to_string()),
        );
        assert!(matches!(err, BedrockError::AccessDenied { .. }));
        assert!(err.is_access_denied());

        // Test Throttling
        let err = BedrockError::from_aws_error(
            "ThrottlingException",
            "Rate exceeded",
            reqwest::StatusCode::TOO_MANY_REQUESTS,
            None,
        );
        assert!(matches!(err, BedrockError::Throttling { .. }));
        assert!(err.is_rate_limit());
    }
}
