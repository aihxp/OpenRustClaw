//! Error types for the AI21 SDK.

use std::fmt;

/// Result type alias for AI21 operations.
pub type Result<T> = std::result::Result<T, Ai21Error>;

/// Errors that can occur when using the AI21 API.
#[derive(Debug)]
pub enum Ai21Error {
    /// Authentication failed (invalid API key).
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
    },

    /// Model not found.
    ModelNotFound {
        /// Model identifier
        model: String,
    },

    /// API error from AI21.
    Api {
        /// HTTP status code
        status: reqwest::StatusCode,
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
        last_error: Box<Ai21Error>,
    },

    /// Internal error.
    Internal {
        /// Error message
        message: String,
    },
}

impl fmt::Display for Ai21Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Ai21Error::Authentication { message } => {
                write!(f, "Authentication failed: {message}")
            }
            Ai21Error::RateLimit {
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
            Ai21Error::InvalidRequest { message } => {
                write!(f, "Invalid request: {message}")
            }
            Ai21Error::ModelNotFound { model } => {
                write!(f, "Model not found: {model}")
            }
            Ai21Error::Api { status, message } => {
                write!(f, "API error ({status}): {message}")
            }
            Ai21Error::Http { source } => {
                write!(f, "HTTP error: {source}")
            }
            Ai21Error::Json { source } => {
                write!(f, "JSON error: {source}")
            }
            Ai21Error::Stream { message } => {
                write!(f, "Stream error: {message}")
            }
            Ai21Error::Config { message } => {
                write!(f, "Configuration error: {message}")
            }
            Ai21Error::Timeout { operation } => {
                write!(f, "Timeout during {operation}")
            }
            Ai21Error::RetryExhausted {
                attempts,
                last_error,
            } => {
                write!(f, "Retry exhausted after {attempts} attempts: {last_error}")
            }
            Ai21Error::Internal { message } => {
                write!(f, "Internal error: {message}")
            }
        }
    }
}

impl std::error::Error for Ai21Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Ai21Error::Http { source } => Some(source),
            Ai21Error::Json { source } => Some(source),
            Ai21Error::RetryExhausted { last_error, .. } => Some(last_error),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for Ai21Error {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            Ai21Error::Timeout {
                operation: "HTTP request".to_string(),
            }
        } else {
            Ai21Error::Http { source: err }
        }
    }
}

impl From<serde_json::Error> for Ai21Error {
    fn from(err: serde_json::Error) -> Self {
        Ai21Error::Json { source: err }
    }
}

impl From<std::io::Error> for Ai21Error {
    fn from(err: std::io::Error) -> Self {
        Ai21Error::Internal {
            message: format!("IO error: {err}"),
        }
    }
}

impl Ai21Error {
    /// Check if this error is retryable.
    pub fn is_retryable(&self) -> bool {
        match self {
            Ai21Error::RateLimit { .. } | Ai21Error::Http { .. } => true,
            Ai21Error::Api { status, .. } => {
                status.is_server_error() || *status == reqwest::StatusCode::TOO_MANY_REQUESTS
            }
            _ => false,
        }
    }

    /// Check if this is a rate limit error.
    pub fn is_rate_limit(&self) -> bool {
        matches!(self, Ai21Error::RateLimit { .. })
    }

    /// Check if this is an authentication error.
    pub fn is_auth_error(&self) -> bool {
        matches!(self, Ai21Error::Authentication { .. })
    }

    /// Get the retry-after duration if available.
    pub fn retry_after(&self) -> Option<std::time::Duration> {
        match self {
            Ai21Error::RateLimit { retry_after, .. } => *retry_after,
            _ => None,
        }
    }

    /// Create an error from an HTTP response.
    pub(crate) async fn from_response(response: reqwest::Response) -> Self {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        // Try to parse AI21's error format
        if let Ok(error_json) = serde_json::from_str::<serde_json::Value>(&body) {
            // AI21 may return errors in different formats
            let message = error_json
                .get("detail")
                .and_then(|v| v.as_str())
                .or_else(|| error_json.get("message").and_then(|v| v.as_str()))
                .or_else(|| error_json.get("error").and_then(|v| v.as_str()))
                .unwrap_or(&body);

            // Map specific HTTP status codes to error types
            match status {
                reqwest::StatusCode::UNAUTHORIZED => {
                    return Ai21Error::Authentication {
                        message: message.to_string(),
                    };
                }
                reqwest::StatusCode::TOO_MANY_REQUESTS => {
                    return Ai21Error::RateLimit {
                        retry_after: None,
                        message: message.to_string(),
                    };
                }
                reqwest::StatusCode::BAD_REQUEST => {
                    return Ai21Error::InvalidRequest {
                        message: message.to_string(),
                    };
                }
                reqwest::StatusCode::NOT_FOUND => {
                    return Ai21Error::ModelNotFound {
                        model: message.to_string(),
                    };
                }
                _ => {}
            }

            return Ai21Error::Api {
                status,
                message: message.to_string(),
            };
        }

        // Fallback for non-JSON responses
        Ai21Error::Api {
            status,
            message: body,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = Ai21Error::Authentication {
            message: "Invalid API key".to_string(),
        };
        assert!(err.to_string().contains("Authentication failed"));
    }

    #[test]
    fn test_is_retryable() {
        let rate_limit = Ai21Error::RateLimit {
            retry_after: None,
            message: "Too many requests".to_string(),
        };
        assert!(rate_limit.is_retryable());

        let auth = Ai21Error::Authentication {
            message: "Invalid key".to_string(),
        };
        assert!(!auth.is_retryable());
    }
}
