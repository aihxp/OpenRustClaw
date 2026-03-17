//! Error types for the Replicate SDK.

use std::fmt;

/// Result type alias for Replicate operations.
pub type Result<T> = std::result::Result<T, ReplicateError>;

/// Errors that can occur when using the Replicate API.
#[derive(Debug)]
pub enum ReplicateError {
    /// Authentication failed (invalid API token).
    Authentication {
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

    /// Invalid request (bad parameters).
    InvalidRequest {
        /// Error message
        message: String,
        /// Parameter that caused the error
        param: Option<String>,
    },

    /// Prediction failed.
    PredictionFailed {
        /// Prediction ID
        id: String,
        /// Error message
        message: String,
    },

    /// Prediction was cancelled.
    PredictionCancelled {
        /// Prediction ID
        id: String,
    },

    /// Model not found or doesn't exist.
    ModelNotFound {
        /// Model identifier
        model: String,
    },

    /// Version not found.
    VersionNotFound {
        /// Version ID
        version: String,
    },

    /// API error from Replicate.
    Api {
        /// HTTP status code
        status: reqwest::StatusCode,
        /// Error code from Replicate
        code: Option<String>,
        /// Error message
        message: String,
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
        last_error: Box<ReplicateError>,
    },

    /// Not found (404).
    NotFound {
        /// Resource type
        resource: String,
        /// Resource ID
        id: String,
    },

    /// Internal error.
    Internal {
        /// Error message
        message: String,
    },

    /// I/O error.
    Io {
        /// Source error
        source: std::io::Error,
    },

    /// Webhook verification error.
    #[cfg(feature = "webhooks")]
    Webhook {
        /// Error message
        message: String,
    },

    /// Invalid header value.
    InvalidHeader {
        /// Error message
        message: String,
    },
}

impl fmt::Display for ReplicateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReplicateError::Authentication { message } => {
                write!(f, "Authentication failed: {message}")
            }
            ReplicateError::RateLimit {
                retry_after,
                message,
            } => {
                if let Some(duration) = retry_after {
                    write!(
                        f,
                        "Rate limit exceeded (retry after {duration:?}): {message}"
                    )
                } else {
                    write!(f, "Rate limit exceeded: {message}")
                }
            }
            ReplicateError::InvalidRequest { message, param } => {
                if let Some(p) = param {
                    write!(f, "Invalid request ({p}): {message}")
                } else {
                    write!(f, "Invalid request: {message}")
                }
            }
            ReplicateError::PredictionFailed { id, message } => {
                write!(f, "Prediction {id} failed: {message}")
            }
            ReplicateError::PredictionCancelled { id } => {
                write!(f, "Prediction {id} was cancelled")
            }
            ReplicateError::ModelNotFound { model } => {
                write!(f, "Model not found: {model}")
            }
            ReplicateError::VersionNotFound { version } => {
                write!(f, "Version not found: {version}")
            }
            ReplicateError::Api {
                status,
                code,
                message,
            } => {
                if let Some(c) = code {
                    write!(f, "API error ({status} - {c}): {message}")
                } else {
                    write!(f, "API error ({status}): {message}")
                }
            }
            ReplicateError::Http { source } => {
                write!(f, "HTTP error: {source}")
            }
            ReplicateError::Json { source } => {
                write!(f, "JSON error: {source}")
            }
            ReplicateError::Stream { message } => {
                write!(f, "Stream error: {message}")
            }
            ReplicateError::Config { message } => {
                write!(f, "Configuration error: {message}")
            }
            ReplicateError::Timeout { operation } => {
                write!(f, "Timeout during {operation}")
            }
            ReplicateError::RetryExhausted {
                attempts,
                last_error,
            } => {
                write!(f, "Retry exhausted after {attempts} attempts: {last_error}")
            }
            ReplicateError::NotFound { resource, id } => {
                write!(f, "{resource} not found: {id}")
            }
            ReplicateError::Internal { message } => {
                write!(f, "Internal error: {message}")
            }
            ReplicateError::Io { source } => {
                write!(f, "I/O error: {source}")
            }
            #[cfg(feature = "webhooks")]
            ReplicateError::Webhook { message } => {
                write!(f, "Webhook error: {message}")
            }
            ReplicateError::InvalidHeader { message } => {
                write!(f, "Invalid header: {message}")
            }
        }
    }
}

impl std::error::Error for ReplicateError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ReplicateError::Http { source } => Some(source),
            ReplicateError::Json { source } => Some(source),
            ReplicateError::RetryExhausted { last_error, .. } => Some(last_error),
            ReplicateError::Io { source } => Some(source),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for ReplicateError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            ReplicateError::Timeout {
                operation: "HTTP request".to_string(),
            }
        } else {
            ReplicateError::Http { source: err }
        }
    }
}

impl From<serde_json::Error> for ReplicateError {
    fn from(err: serde_json::Error) -> Self {
        ReplicateError::Json { source: err }
    }
}

impl From<std::io::Error> for ReplicateError {
    fn from(err: std::io::Error) -> Self {
        ReplicateError::Io { source: err }
    }
}

impl From<reqwest::header::InvalidHeaderValue> for ReplicateError {
    fn from(_: reqwest::header::InvalidHeaderValue) -> Self {
        ReplicateError::InvalidHeader {
            message: "Invalid header value".to_string(),
        }
    }
}

impl ReplicateError {
    /// Check if this error is retryable.
    pub fn is_retryable(&self) -> bool {
        match self {
            ReplicateError::RateLimit { .. } | ReplicateError::Http { .. } => true,
            ReplicateError::Api { status, .. } => {
                status.is_server_error() || *status == reqwest::StatusCode::TOO_MANY_REQUESTS
            }
            _ => false,
        }
    }

    /// Check if this is a rate limit error.
    pub fn is_rate_limit(&self) -> bool {
        matches!(self, ReplicateError::RateLimit { .. })
    }

    /// Check if this is an authentication error.
    pub fn is_auth_error(&self) -> bool {
        matches!(self, ReplicateError::Authentication { .. })
    }

    /// Get the retry-after duration if available.
    pub fn retry_after(&self) -> Option<std::time::Duration> {
        match self {
            ReplicateError::RateLimit { retry_after, .. } => *retry_after,
            _ => None,
        }
    }

    /// Create an error from an HTTP response.
    pub(crate) async fn from_response(response: reqwest::Response) -> Self {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        // Try to parse Replicate's error format
        if let Ok(error_json) = serde_json::from_str::<serde_json::Value>(&body) {
            if let Some(error_obj) = error_json.get("error") {
                let code = error_obj
                    .get("code")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let message = error_obj
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&body)
                    .to_string();
                let param = error_obj
                    .get("param")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                // Map specific error codes
                match code.as_deref() {
                    Some("invalid_token") | Some("unauthorized") => {
                        return ReplicateError::Authentication { message };
                    }
                    Some("rate_limit_exceeded") => {
                        return ReplicateError::RateLimit {
                            retry_after: None,
                            message,
                        };
                    }
                    Some("invalid_request") => {
                        return ReplicateError::InvalidRequest { message, param };
                    }
                    Some("model_not_found") => {
                        return ReplicateError::ModelNotFound { model: message };
                    }
                    Some("version_not_found") => {
                        return ReplicateError::VersionNotFound { version: message };
                    }
                    _ => {}
                }

                return ReplicateError::Api {
                    status,
                    code,
                    message,
                };
            }
        }

        // Fallback for non-JSON responses
        ReplicateError::Api {
            status,
            code: None,
            message: body,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = ReplicateError::Authentication {
            message: "Invalid token".to_string(),
        };
        assert!(err.to_string().contains("Authentication failed"));
    }

    #[test]
    fn test_is_retryable() {
        let rate_limit = ReplicateError::RateLimit {
            retry_after: None,
            message: "Too many requests".to_string(),
        };
        assert!(rate_limit.is_retryable());

        let auth = ReplicateError::Authentication {
            message: "Invalid token".to_string(),
        };
        assert!(!auth.is_retryable());
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let rep_err: ReplicateError = io_err.into();
        assert!(matches!(rep_err, ReplicateError::Io { .. }));
    }
}
