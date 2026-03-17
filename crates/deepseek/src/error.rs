//! Error types for the DeepSeek SDK.

use std::fmt;

/// Result type alias for DeepSeek operations.
pub type Result<T> = std::result::Result<T, DeepSeekError>;

/// Errors that can occur when using the DeepSeek API.
#[derive(Debug)]
pub enum DeepSeekError {
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
        /// Parameter that caused the error
        param: Option<String>,
    },

    /// Context length exceeded.
    ContextLength {
        /// Tokens used
        tokens_used: usize,
        /// Maximum tokens allowed
        max_tokens: usize,
    },

    /// Model not found or doesn't exist.
    ModelNotFound {
        /// Model identifier
        model: String,
    },

    /// API error from DeepSeek.
    Api {
        /// HTTP status code
        status: reqwest::StatusCode,
        /// Error code from DeepSeek
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
        last_error: Box<DeepSeekError>,
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
}

impl fmt::Display for DeepSeekError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeepSeekError::Authentication { message } => {
                write!(f, "Authentication failed: {message}")
            }
            DeepSeekError::RateLimit { retry_after, message } => {
                if let Some(duration) = retry_after {
                    write!(f, "Rate limit exceeded (retry after {duration:?}): {message}")
                } else {
                    write!(f, "Rate limit exceeded: {message}")
                }
            }
            DeepSeekError::InvalidRequest { message, param } => {
                if let Some(p) = param {
                    write!(f, "Invalid request ({p}): {message}")
                } else {
                    write!(f, "Invalid request: {message}")
                }
            }
            DeepSeekError::ContextLength {
                tokens_used,
                max_tokens,
            } => {
                write!(
                    f,
                    "Context length exceeded: {tokens_used} tokens used, {max_tokens} max"
                )
            }
            DeepSeekError::ModelNotFound { model } => {
                write!(f, "Model not found: {model}")
            }
            DeepSeekError::Api {
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
            DeepSeekError::Http { source } => {
                write!(f, "HTTP error: {source}")
            }
            DeepSeekError::Json { source } => {
                write!(f, "JSON error: {source}")
            }
            DeepSeekError::Stream { message } => {
                write!(f, "Stream error: {message}")
            }
            DeepSeekError::Config { message } => {
                write!(f, "Configuration error: {message}")
            }
            DeepSeekError::Timeout { operation } => {
                write!(f, "Timeout during {operation}")
            }
            DeepSeekError::RetryExhausted { attempts, last_error } => {
                write!(f, "Retry exhausted after {attempts} attempts: {last_error}")
            }
            DeepSeekError::NotFound { resource, id } => {
                write!(f, "{resource} not found: {id}")
            }
            DeepSeekError::Internal { message } => {
                write!(f, "Internal error: {message}")
            }
            DeepSeekError::Io { source } => {
                write!(f, "I/O error: {source}")
            }
        }
    }
}

impl std::error::Error for DeepSeekError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DeepSeekError::Http { source } => Some(source),
            DeepSeekError::Json { source } => Some(source),
            DeepSeekError::RetryExhausted { last_error, .. } => Some(last_error),
            DeepSeekError::Io { source } => Some(source),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for DeepSeekError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            DeepSeekError::Timeout {
                operation: "HTTP request".to_string(),
            }
        } else {
            DeepSeekError::Http { source: err }
        }
    }
}

impl From<serde_json::Error> for DeepSeekError {
    fn from(err: serde_json::Error) -> Self {
        DeepSeekError::Json { source: err }
    }
}

impl From<std::io::Error> for DeepSeekError {
    fn from(err: std::io::Error) -> Self {
        DeepSeekError::Io { source: err }
    }
}

impl DeepSeekError {
    /// Check if this error is retryable.
    pub fn is_retryable(&self) -> bool {
        match self {
            DeepSeekError::RateLimit { .. } | DeepSeekError::Http { .. } => true,
            DeepSeekError::Api { status, .. } => {
                status.is_server_error() || *status == reqwest::StatusCode::TOO_MANY_REQUESTS
            }
            _ => false,
        }
    }

    /// Check if this is a rate limit error.
    pub fn is_rate_limit(&self) -> bool {
        matches!(self, DeepSeekError::RateLimit { .. })
    }

    /// Check if this is an authentication error.
    pub fn is_auth_error(&self) -> bool {
        matches!(self, DeepSeekError::Authentication { .. })
    }

    /// Get the retry-after duration if available.
    pub fn retry_after(&self) -> Option<std::time::Duration> {
        match self {
            DeepSeekError::RateLimit { retry_after, .. } => *retry_after,
            _ => None,
        }
    }

    /// Create an error from an HTTP response.
    pub(crate) async fn from_response(response: reqwest::Response) -> Self {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        // Try to parse DeepSeek's error format
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
                    Some("invalid_api_key") | Some("unauthorized") => {
                        return DeepSeekError::Authentication { message };
                    }
                    Some("rate_limit_exceeded") => {
                        return DeepSeekError::RateLimit {
                            retry_after: None,
                            message,
                        };
                    }
                    Some("insufficient_quota") => {
                        return DeepSeekError::RateLimit {
                            retry_after: None,
                            message,
                        };
                    }
                    Some("invalid_request_error") => {
                        return DeepSeekError::InvalidRequest { message, param };
                    }
                    Some("context_length_exceeded") => {
                        return DeepSeekError::ContextLength {
                            tokens_used: 0,
                            max_tokens: 0,
                        };
                    }
                    Some("model_not_found") => {
                        return DeepSeekError::ModelNotFound { model: message };
                    }
                    _ => {}
                }

                return DeepSeekError::Api {
                    status,
                    code,
                    message,
                };
            }
        }

        // Fallback for non-JSON responses
        DeepSeekError::Api {
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
        let err = DeepSeekError::Authentication {
            message: "Invalid API key".to_string(),
        };
        assert!(err.to_string().contains("Authentication failed"));
    }

    #[test]
    fn test_is_retryable() {
        let rate_limit = DeepSeekError::RateLimit {
            retry_after: None,
            message: "Too many requests".to_string(),
        };
        assert!(rate_limit.is_retryable());

        let auth = DeepSeekError::Authentication {
            message: "Invalid key".to_string(),
        };
        assert!(!auth.is_retryable());
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let deepseek_err: DeepSeekError = io_err.into();
        assert!(matches!(deepseek_err, DeepSeekError::Io { .. }));
    }
}
