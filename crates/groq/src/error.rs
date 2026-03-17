//! Error types for the Groq SDK.

use std::fmt;

/// Result type alias for Groq operations.
pub type Result<T> = std::result::Result<T, GroqError>;

/// Errors that can occur when using the Groq API.
#[derive(Debug)]
pub enum GroqError {
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

    /// API error from Groq.
    Api {
        /// HTTP status code
        status: reqwest::StatusCode,
        /// Error code from Groq
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
        last_error: Box<GroqError>,
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

    /// I/O error (for file operations in audio transcription).
    Io {
        /// Source error
        source: std::io::Error,
    },

    /// Audio file error.
    Audio {
        /// Error message
        message: String,
    },
}

impl fmt::Display for GroqError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GroqError::Authentication { message } => {
                write!(f, "Authentication failed: {message}")
            }
            GroqError::RateLimit {
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
            GroqError::InvalidRequest { message, param } => {
                if let Some(p) = param {
                    write!(f, "Invalid request ({p}): {message}")
                } else {
                    write!(f, "Invalid request: {message}")
                }
            }
            GroqError::ContextLength {
                tokens_used,
                max_tokens,
            } => {
                write!(
                    f,
                    "Context length exceeded: {tokens_used} tokens used, {max_tokens} max"
                )
            }
            GroqError::ModelNotFound { model } => {
                write!(f, "Model not found: {model}")
            }
            GroqError::Api {
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
            GroqError::Http { source } => {
                write!(f, "HTTP error: {source}")
            }
            GroqError::Json { source } => {
                write!(f, "JSON error: {source}")
            }
            GroqError::Stream { message } => {
                write!(f, "Stream error: {message}")
            }
            GroqError::Config { message } => {
                write!(f, "Configuration error: {message}")
            }
            GroqError::Timeout { operation } => {
                write!(f, "Timeout during {operation}")
            }
            GroqError::RetryExhausted {
                attempts,
                last_error,
            } => {
                write!(f, "Retry exhausted after {attempts} attempts: {last_error}")
            }
            GroqError::NotFound { resource, id } => {
                write!(f, "{resource} not found: {id}")
            }
            GroqError::Internal { message } => {
                write!(f, "Internal error: {message}")
            }
            GroqError::Io { source } => {
                write!(f, "I/O error: {source}")
            }
            GroqError::Audio { message } => {
                write!(f, "Audio error: {message}")
            }
        }
    }
}

impl std::error::Error for GroqError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            GroqError::Http { source } => Some(source),
            GroqError::Json { source } => Some(source),
            GroqError::RetryExhausted { last_error, .. } => Some(last_error),
            GroqError::Io { source } => Some(source),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for GroqError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            GroqError::Timeout {
                operation: "HTTP request".to_string(),
            }
        } else {
            GroqError::Http { source: err }
        }
    }
}

impl From<serde_json::Error> for GroqError {
    fn from(err: serde_json::Error) -> Self {
        GroqError::Json { source: err }
    }
}

impl From<std::io::Error> for GroqError {
    fn from(err: std::io::Error) -> Self {
        GroqError::Io { source: err }
    }
}

impl GroqError {
    /// Check if this error is retryable.
    pub fn is_retryable(&self) -> bool {
        match self {
            GroqError::RateLimit { .. } | GroqError::Http { .. } => true,
            GroqError::Api { status, .. } => {
                status.is_server_error() || *status == reqwest::StatusCode::TOO_MANY_REQUESTS
            }
            _ => false,
        }
    }

    /// Check if this is a rate limit error.
    pub fn is_rate_limit(&self) -> bool {
        matches!(self, GroqError::RateLimit { .. })
    }

    /// Check if this is an authentication error.
    pub fn is_auth_error(&self) -> bool {
        matches!(self, GroqError::Authentication { .. })
    }

    /// Get the retry-after duration if available.
    pub fn retry_after(&self) -> Option<std::time::Duration> {
        match self {
            GroqError::RateLimit { retry_after, .. } => *retry_after,
            _ => None,
        }
    }

    /// Create an error from an HTTP response.
    pub(crate) async fn from_response(response: reqwest::Response) -> Self {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        // Try to parse Groq's error format
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
                        return GroqError::Authentication { message };
                    }
                    Some("rate_limit_exceeded") => {
                        return GroqError::RateLimit {
                            retry_after: None,
                            message,
                        };
                    }
                    Some("insufficient_quota") => {
                        return GroqError::RateLimit {
                            retry_after: None,
                            message,
                        };
                    }
                    Some("invalid_request_error") => {
                        return GroqError::InvalidRequest { message, param };
                    }
                    Some("context_length_exceeded") => {
                        return GroqError::ContextLength {
                            tokens_used: 0,
                            max_tokens: 0,
                        };
                    }
                    Some("model_not_found") => {
                        return GroqError::ModelNotFound { model: message };
                    }
                    _ => {}
                }

                return GroqError::Api {
                    status,
                    code,
                    message,
                };
            }
        }

        // Fallback for non-JSON responses
        GroqError::Api {
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
        let err = GroqError::Authentication {
            message: "Invalid API key".to_string(),
        };
        assert!(err.to_string().contains("Authentication failed"));
    }

    #[test]
    fn test_is_retryable() {
        let rate_limit = GroqError::RateLimit {
            retry_after: None,
            message: "Too many requests".to_string(),
        };
        assert!(rate_limit.is_retryable());

        let auth = GroqError::Authentication {
            message: "Invalid key".to_string(),
        };
        assert!(!auth.is_retryable());
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let groq_err: GroqError = io_err.into();
        assert!(matches!(groq_err, GroqError::Io { .. }));
    }
}
