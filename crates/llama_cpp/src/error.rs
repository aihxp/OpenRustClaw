//! Error types for the llama.cpp SDK.

use std::fmt;

/// Result type alias for llama.cpp operations.
pub type Result<T> = std::result::Result<T, LlamaCppError>;

/// Errors that can occur when using the llama.cpp API.
#[derive(Debug)]
pub enum LlamaCppError {
    /// Server is not available or not responding.
    ServerUnavailable {
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

    /// Slot not available for processing.
    SlotUnavailable {
        /// Error message
        message: String,
    },

    /// API error from llama.cpp server.
    Api {
        /// HTTP status code
        status: reqwest::StatusCode,
        /// Error code from server
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
        last_error: Box<LlamaCppError>,
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

impl fmt::Display for LlamaCppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LlamaCppError::ServerUnavailable { message } => {
                write!(f, "Server unavailable: {message}")
            }
            LlamaCppError::InvalidRequest { message, param } => {
                if let Some(p) = param {
                    write!(f, "Invalid request ({p}): {message}")
                } else {
                    write!(f, "Invalid request: {message}")
                }
            }
            LlamaCppError::ContextLength {
                tokens_used,
                max_tokens,
            } => {
                write!(
                    f,
                    "Context length exceeded: {tokens_used} tokens used, {max_tokens} max"
                )
            }
            LlamaCppError::ModelNotFound { model } => {
                write!(f, "Model not found: {model}")
            }
            LlamaCppError::SlotUnavailable { message } => {
                write!(f, "Slot unavailable: {message}")
            }
            LlamaCppError::Api {
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
            LlamaCppError::Http { source } => {
                write!(f, "HTTP error: {source}")
            }
            LlamaCppError::Json { source } => {
                write!(f, "JSON error: {source}")
            }
            LlamaCppError::Stream { message } => {
                write!(f, "Stream error: {message}")
            }
            LlamaCppError::Config { message } => {
                write!(f, "Configuration error: {message}")
            }
            LlamaCppError::Timeout { operation } => {
                write!(f, "Timeout during {operation}")
            }
            LlamaCppError::RetryExhausted {
                attempts,
                last_error,
            } => {
                write!(f, "Retry exhausted after {attempts} attempts: {last_error}")
            }
            LlamaCppError::NotFound { resource, id } => {
                write!(f, "{resource} not found: {id}")
            }
            LlamaCppError::Internal { message } => {
                write!(f, "Internal error: {message}")
            }
            LlamaCppError::Io { source } => {
                write!(f, "I/O error: {source}")
            }
        }
    }
}

impl std::error::Error for LlamaCppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LlamaCppError::Http { source } => Some(source),
            LlamaCppError::Json { source } => Some(source),
            LlamaCppError::RetryExhausted { last_error, .. } => Some(last_error),
            LlamaCppError::Io { source } => Some(source),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for LlamaCppError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            LlamaCppError::Timeout {
                operation: "HTTP request".to_string(),
            }
        } else if err.is_connect() {
            LlamaCppError::ServerUnavailable {
                message: format!("Failed to connect to server: {err}"),
            }
        } else {
            LlamaCppError::Http { source: err }
        }
    }
}

impl From<serde_json::Error> for LlamaCppError {
    fn from(err: serde_json::Error) -> Self {
        LlamaCppError::Json { source: err }
    }
}

impl From<std::io::Error> for LlamaCppError {
    fn from(err: std::io::Error) -> Self {
        LlamaCppError::Io { source: err }
    }
}

impl LlamaCppError {
    /// Check if this error is retryable.
    pub fn is_retryable(&self) -> bool {
        match self {
            LlamaCppError::ServerUnavailable { .. }
            | LlamaCppError::SlotUnavailable { .. }
            | LlamaCppError::Http { .. }
            | LlamaCppError::Timeout { .. } => true,
            LlamaCppError::Api { status, .. } => {
                status.is_server_error() || *status == reqwest::StatusCode::TOO_MANY_REQUESTS
            }
            _ => false,
        }
    }

    /// Check if this is a server unavailable error.
    pub fn is_server_unavailable(&self) -> bool {
        matches!(self, LlamaCppError::ServerUnavailable { .. })
    }

    /// Check if this is a slot unavailable error.
    pub fn is_slot_unavailable(&self) -> bool {
        matches!(self, LlamaCppError::SlotUnavailable { .. })
    }

    /// Create an error from an HTTP response.
    pub(crate) async fn from_response(response: reqwest::Response) -> Self {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        // Try to parse llama.cpp's error format
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
                    Some("context_length_exceeded") => {
                        return LlamaCppError::ContextLength {
                            tokens_used: 0,
                            max_tokens: 0,
                        };
                    }
                    Some("invalid_request") => {
                        return LlamaCppError::InvalidRequest { message, param };
                    }
                    Some("slot_unavailable") => {
                        return LlamaCppError::SlotUnavailable { message };
                    }
                    _ => {}
                }

                return LlamaCppError::Api {
                    status,
                    code,
                    message,
                };
            }

            // llama.cpp sometimes returns error as a simple string
            if let Some(message) = error_json.as_str() {
                return LlamaCppError::Api {
                    status,
                    code: None,
                    message: message.to_string(),
                };
            }
        }

        // Fallback for non-JSON responses
        LlamaCppError::Api {
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
        let err = LlamaCppError::ServerUnavailable {
            message: "Connection refused".to_string(),
        };
        assert!(err.to_string().contains("Server unavailable"));
    }

    #[test]
    fn test_is_retryable() {
        let server_unavailable = LlamaCppError::ServerUnavailable {
            message: "Connection refused".to_string(),
        };
        assert!(server_unavailable.is_retryable());

        let invalid_request = LlamaCppError::InvalidRequest {
            message: "Bad request".to_string(),
            param: None,
        };
        assert!(!invalid_request.is_retryable());
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let llama_err: LlamaCppError = io_err.into();
        assert!(matches!(llama_err, LlamaCppError::Io { .. }));
    }
}
