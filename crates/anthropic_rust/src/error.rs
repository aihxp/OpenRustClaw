//! Error types for the Anthropic SDK.

use std::fmt;

/// Result type alias for Anthropic operations.
pub type Result<T> = std::result::Result<T, AnthropicError>;

/// Errors that can occur when using the Anthropic API.
#[derive(Debug)]
pub enum AnthropicError {
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

    /// Context length exceeded.
    ContextLength {
        /// Tokens used
        tokens_used: usize,
        /// Maximum tokens allowed
        max_tokens: usize,
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

    /// API error from Anthropic.
    Api {
        /// HTTP status code
        status: reqwest::StatusCode,
        /// Error type from Anthropic
        error_type: Option<String>,
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
        last_error: Box<AnthropicError>,
    },

    /// Internal error.
    Internal {
        /// Error message
        message: String,
    },
}

impl fmt::Display for AnthropicError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnthropicError::Authentication { message } => {
                write!(f, "Authentication failed: {message}")
            }
            AnthropicError::RateLimit {
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
            AnthropicError::ContextLength {
                tokens_used,
                max_tokens,
            } => {
                write!(
                    f,
                    "Context length exceeded: {tokens_used} tokens used, {max_tokens} max"
                )
            }
            AnthropicError::InvalidRequest { message } => {
                write!(f, "Invalid request: {message}")
            }
            AnthropicError::ModelNotFound { model } => {
                write!(f, "Model not found: {model}")
            }
            AnthropicError::Api {
                status,
                error_type,
                message,
            } => {
                if let Some(etype) = error_type {
                    write!(f, "API error ({status} - {etype}): {message}")
                } else {
                    write!(f, "API error ({status}): {message}")
                }
            }
            AnthropicError::Http { source } => {
                write!(f, "HTTP error: {source}")
            }
            AnthropicError::Json { source } => {
                write!(f, "JSON error: {source}")
            }
            AnthropicError::Stream { message } => {
                write!(f, "Stream error: {message}")
            }
            AnthropicError::Config { message } => {
                write!(f, "Configuration error: {message}")
            }
            AnthropicError::Timeout { operation } => {
                write!(f, "Timeout during {operation}")
            }
            AnthropicError::RetryExhausted {
                attempts,
                last_error,
            } => {
                write!(f, "Retry exhausted after {attempts} attempts: {last_error}")
            }
            AnthropicError::Internal { message } => {
                write!(f, "Internal error: {message}")
            }
        }
    }
}

impl std::error::Error for AnthropicError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AnthropicError::Http { source } => Some(source),
            AnthropicError::Json { source } => Some(source),
            AnthropicError::RetryExhausted { last_error, .. } => Some(last_error),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for AnthropicError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            AnthropicError::Timeout {
                operation: "HTTP request".to_string(),
            }
        } else {
            AnthropicError::Http { source: err }
        }
    }
}

impl From<serde_json::Error> for AnthropicError {
    fn from(err: serde_json::Error) -> Self {
        AnthropicError::Json { source: err }
    }
}

impl From<std::io::Error> for AnthropicError {
    fn from(err: std::io::Error) -> Self {
        AnthropicError::Internal {
            message: format!("IO error: {err}"),
        }
    }
}

impl AnthropicError {
    /// Check if this error is retryable.
    pub fn is_retryable(&self) -> bool {
        match self {
            AnthropicError::RateLimit { .. } | AnthropicError::Http { .. } => true,
            AnthropicError::Api { status, .. } => {
                status.is_server_error() || *status == reqwest::StatusCode::TOO_MANY_REQUESTS
            }
            _ => false,
        }
    }

    /// Check if this is a rate limit error.
    pub fn is_rate_limit(&self) -> bool {
        matches!(self, AnthropicError::RateLimit { .. })
    }

    /// Check if this is an authentication error.
    pub fn is_auth_error(&self) -> bool {
        matches!(self, AnthropicError::Authentication { .. })
    }

    /// Get the retry-after duration if available.
    pub fn retry_after(&self) -> Option<std::time::Duration> {
        match self {
            AnthropicError::RateLimit { retry_after, .. } => *retry_after,
            _ => None,
        }
    }

    /// Create an error from an HTTP response.
    pub(crate) async fn from_response(response: reqwest::Response) -> Self {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        // Try to parse Anthropic's error format
        if let Ok(error_json) = serde_json::from_str::<serde_json::Value>(&body) {
            if let Some(error_obj) = error_json.get("error") {
                let error_type = error_obj
                    .get("type")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let message = error_obj
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&body)
                    .to_string();

                // Map specific error types
                match error_type.as_deref() {
                    Some("authentication_error") => {
                        return AnthropicError::Authentication { message };
                    }
                    Some("rate_limit_error") => {
                        return AnthropicError::RateLimit {
                            retry_after: None,
                            message,
                        };
                    }
                    Some("invalid_request_error") => {
                        return AnthropicError::InvalidRequest { message };
                    }
                    Some("context_length_exceeded") => {
                        return AnthropicError::ContextLength {
                            tokens_used: 0,
                            max_tokens: 0,
                        };
                    }
                    Some("not_found_error") => {
                        return AnthropicError::ModelNotFound {
                            model: message.clone(),
                        };
                    }
                    _ => {}
                }

                return AnthropicError::Api {
                    status,
                    error_type,
                    message,
                };
            }
        }

        // Fallback for non-JSON responses
        AnthropicError::Api {
            status,
            error_type: None,
            message: body,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = AnthropicError::Authentication {
            message: "Invalid API key".to_string(),
        };
        assert!(err.to_string().contains("Authentication failed"));
    }

    #[test]
    fn test_is_retryable() {
        let rate_limit = AnthropicError::RateLimit {
            retry_after: None,
            message: "Too many requests".to_string(),
        };
        assert!(rate_limit.is_retryable());

        let auth = AnthropicError::Authentication {
            message: "Invalid key".to_string(),
        };
        assert!(!auth.is_retryable());
    }
}
