//! Error types for the Perplexity SDK.

use std::fmt;

/// Result type alias for Perplexity operations.
pub type Result<T> = std::result::Result<T, PerplexityError>;

/// Errors that can occur when using the Perplexity API.
#[derive(Debug)]
pub enum PerplexityError {
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

    /// API error from Perplexity.
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
        last_error: Box<PerplexityError>,
    },

    /// Internal error.
    Internal {
        /// Error message
        message: String,
    },
}

impl fmt::Display for PerplexityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PerplexityError::Authentication { message } => {
                write!(f, "Authentication failed: {message}")
            }
            PerplexityError::RateLimit {
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
            PerplexityError::InvalidRequest { message } => {
                write!(f, "Invalid request: {message}")
            }
            PerplexityError::ModelNotFound { model } => {
                write!(f, "Model not found: {model}")
            }
            PerplexityError::Api { status, message } => {
                write!(f, "API error ({status}): {message}")
            }
            PerplexityError::Http { source } => {
                write!(f, "HTTP error: {source}")
            }
            PerplexityError::Json { source } => {
                write!(f, "JSON error: {source}")
            }
            PerplexityError::Stream { message } => {
                write!(f, "Stream error: {message}")
            }
            PerplexityError::Config { message } => {
                write!(f, "Configuration error: {message}")
            }
            PerplexityError::Timeout { operation } => {
                write!(f, "Timeout during {operation}")
            }
            PerplexityError::RetryExhausted {
                attempts,
                last_error,
            } => {
                write!(f, "Retry exhausted after {attempts} attempts: {last_error}")
            }
            PerplexityError::Internal { message } => {
                write!(f, "Internal error: {message}")
            }
        }
    }
}

impl std::error::Error for PerplexityError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PerplexityError::Http { source } => Some(source),
            PerplexityError::Json { source } => Some(source),
            PerplexityError::RetryExhausted { last_error, .. } => Some(last_error),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for PerplexityError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            PerplexityError::Timeout {
                operation: "HTTP request".to_string(),
            }
        } else {
            PerplexityError::Http { source: err }
        }
    }
}

impl From<serde_json::Error> for PerplexityError {
    fn from(err: serde_json::Error) -> Self {
        PerplexityError::Json { source: err }
    }
}

impl From<std::io::Error> for PerplexityError {
    fn from(err: std::io::Error) -> Self {
        PerplexityError::Internal {
            message: format!("IO error: {err}"),
        }
    }
}

impl PerplexityError {
    /// Check if this error is retryable.
    pub fn is_retryable(&self) -> bool {
        match self {
            PerplexityError::RateLimit { .. } | PerplexityError::Http { .. } => true,
            PerplexityError::Api { status, .. } => {
                status.is_server_error() || *status == reqwest::StatusCode::TOO_MANY_REQUESTS
            }
            _ => false,
        }
    }

    /// Check if this is a rate limit error.
    pub fn is_rate_limit(&self) -> bool {
        matches!(self, PerplexityError::RateLimit { .. })
    }

    /// Check if this is an authentication error.
    pub fn is_auth_error(&self) -> bool {
        matches!(self, PerplexityError::Authentication { .. })
    }

    /// Get the retry-after duration if available.
    pub fn retry_after(&self) -> Option<std::time::Duration> {
        match self {
            PerplexityError::RateLimit { retry_after, .. } => *retry_after,
            _ => None,
        }
    }

    /// Create an error from an HTTP response.
    pub(crate) async fn from_response(response: reqwest::Response) -> Self {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        // Try to parse Perplexity's error format
        if let Ok(error_json) = serde_json::from_str::<serde_json::Value>(&body) {
            if let Some(error_obj) = error_json.get("error") {
                let message = error_obj
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&body)
                    .to_string();

                // Map specific HTTP status codes to error types
                match status {
                    reqwest::StatusCode::UNAUTHORIZED => {
                        return PerplexityError::Authentication { message };
                    }
                    reqwest::StatusCode::TOO_MANY_REQUESTS => {
                        return PerplexityError::RateLimit {
                            retry_after: None,
                            message,
                        };
                    }
                    reqwest::StatusCode::BAD_REQUEST => {
                        return PerplexityError::InvalidRequest { message };
                    }
                    reqwest::StatusCode::NOT_FOUND => {
                        return PerplexityError::ModelNotFound {
                            model: message.clone(),
                        };
                    }
                    _ => {
                        return PerplexityError::Api { status, message };
                    }
                }
            }

            // Try alternative error format
            if let Some(message) = error_json.get("message").and_then(|v| v.as_str()) {
                match status {
                    reqwest::StatusCode::UNAUTHORIZED => {
                        return PerplexityError::Authentication {
                            message: message.to_string(),
                        };
                    }
                    reqwest::StatusCode::TOO_MANY_REQUESTS => {
                        return PerplexityError::RateLimit {
                            retry_after: None,
                            message: message.to_string(),
                        };
                    }
                    reqwest::StatusCode::BAD_REQUEST => {
                        return PerplexityError::InvalidRequest {
                            message: message.to_string(),
                        };
                    }
                    _ => {
                        return PerplexityError::Api {
                            status,
                            message: message.to_string(),
                        };
                    }
                }
            }
        }

        // Fallback for non-JSON responses
        PerplexityError::Api {
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
        let err = PerplexityError::Authentication {
            message: "Invalid API key".to_string(),
        };
        assert!(err.to_string().contains("Authentication failed"));
    }

    #[test]
    fn test_is_retryable() {
        let rate_limit = PerplexityError::RateLimit {
            retry_after: None,
            message: "Too many requests".to_string(),
        };
        assert!(rate_limit.is_retryable());

        let auth = PerplexityError::Authentication {
            message: "Invalid key".to_string(),
        };
        assert!(!auth.is_retryable());
    }
}
