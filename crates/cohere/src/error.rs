//! Error types for the Cohere SDK.

use std::fmt;

/// Result type alias for Cohere operations.
pub type Result<T> = std::result::Result<T, CohereError>;

/// Errors that can occur when using the Cohere API.
#[derive(Debug)]
pub enum CohereError {
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

    /// API error from Cohere.
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
        last_error: Box<CohereError>,
    },

    /// Internal error.
    Internal {
        /// Error message
        message: String,
    },
}

impl fmt::Display for CohereError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CohereError::Authentication { message } => {
                write!(f, "Authentication failed: {message}")
            }
            CohereError::RateLimit { retry_after, message } => {
                if let Some(duration) = retry_after {
                    write!(f, "Rate limit exceeded (retry after {duration:?}): {message}")
                } else {
                    write!(f, "Rate limit exceeded: {message}")
                }
            }
            CohereError::InvalidRequest { message } => {
                write!(f, "Invalid request: {message}")
            }
            CohereError::ModelNotFound { model } => {
                write!(f, "Model not found: {model}")
            }
            CohereError::Api { status, message } => {
                write!(f, "API error ({status}): {message}")
            }
            CohereError::Http { source } => {
                write!(f, "HTTP error: {source}")
            }
            CohereError::Json { source } => {
                write!(f, "JSON error: {source}")
            }
            CohereError::Stream { message } => {
                write!(f, "Stream error: {message}")
            }
            CohereError::Config { message } => {
                write!(f, "Configuration error: {message}")
            }
            CohereError::Timeout { operation } => {
                write!(f, "Timeout during {operation}")
            }
            CohereError::RetryExhausted { attempts, last_error } => {
                write!(f, "Retry exhausted after {attempts} attempts: {last_error}")
            }
            CohereError::Internal { message } => {
                write!(f, "Internal error: {message}")
            }
        }
    }
}

impl std::error::Error for CohereError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CohereError::Http { source } => Some(source),
            CohereError::Json { source } => Some(source),
            CohereError::RetryExhausted { last_error, .. } => Some(last_error),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for CohereError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            CohereError::Timeout {
                operation: "HTTP request".to_string(),
            }
        } else if err.is_connect() {
            CohereError::Http { source: err }
        } else {
            CohereError::Http { source: err }
        }
    }
}

impl From<serde_json::Error> for CohereError {
    fn from(err: serde_json::Error) -> Self {
        CohereError::Json { source: err }
    }
}

impl From<std::io::Error> for CohereError {
    fn from(err: std::io::Error) -> Self {
        CohereError::Internal {
            message: format!("IO error: {err}"),
        }
    }
}

impl CohereError {
    /// Check if this error is retryable.
    pub fn is_retryable(&self) -> bool {
        match self {
            CohereError::RateLimit { .. } | CohereError::Http { .. } => true,
            CohereError::Api { status, .. } => {
                status.is_server_error() || *status == reqwest::StatusCode::TOO_MANY_REQUESTS
            }
            _ => false,
        }
    }

    /// Check if this is a rate limit error.
    pub fn is_rate_limit(&self) -> bool {
        matches!(self, CohereError::RateLimit { .. })
    }

    /// Check if this is an authentication error.
    pub fn is_auth_error(&self) -> bool {
        matches!(self, CohereError::Authentication { .. })
    }

    /// Get the retry-after duration if available.
    pub fn retry_after(&self) -> Option<std::time::Duration> {
        match self {
            CohereError::RateLimit { retry_after, .. } => *retry_after,
            _ => None,
        }
    }

    /// Create an error from an HTTP response.
    pub(crate) async fn from_response(response: reqwest::Response) -> Self {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        // Try to parse Cohere's error format
        if let Ok(error_json) = serde_json::from_str::<serde_json::Value>(&body) {
            if let Some(message) = error_json
                .get("message")
                .and_then(|v| v.as_str())
            {
                // Map specific HTTP status codes to error types
                match status {
                    reqwest::StatusCode::UNAUTHORIZED => {
                        return CohereError::Authentication {
                            message: message.to_string(),
                        };
                    }
                    reqwest::StatusCode::TOO_MANY_REQUESTS => {
                        return CohereError::RateLimit {
                            retry_after: None,
                            message: message.to_string(),
                        };
                    }
                    reqwest::StatusCode::BAD_REQUEST => {
                        return CohereError::InvalidRequest {
                            message: message.to_string(),
                        };
                    }
                    reqwest::StatusCode::NOT_FOUND => {
                        return CohereError::ModelNotFound {
                            model: message.to_string(),
                        };
                    }
                    _ => {}
                }

                return CohereError::Api {
                    status,
                    message: message.to_string(),
                };
            }
        }

        // Fallback for non-JSON responses
        CohereError::Api {
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
        let err = CohereError::Authentication {
            message: "Invalid API key".to_string(),
        };
        assert!(err.to_string().contains("Authentication failed"));
    }

    #[test]
    fn test_is_retryable() {
        let rate_limit = CohereError::RateLimit {
            retry_after: None,
            message: "Too many requests".to_string(),
        };
        assert!(rate_limit.is_retryable());

        let auth = CohereError::Authentication {
            message: "Invalid key".to_string(),
        };
        assert!(!auth.is_retryable());
    }
}
