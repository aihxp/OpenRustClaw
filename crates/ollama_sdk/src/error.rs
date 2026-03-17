//! Error types for the Ollama SDK.

use std::fmt;

/// Result type alias for Ollama operations.
pub type Result<T> = std::result::Result<T, OllamaError>;

/// Errors that can occur when using the Ollama API.
#[derive(Debug)]
pub enum OllamaError {
    /// Connection error (Ollama not running or unreachable).
    Connection {
        /// Error message
        message: String,
    },

    /// Model not found or doesn't exist.
    ModelNotFound {
        /// Model identifier
        model: String,
    },

    /// Invalid request (bad parameters).
    InvalidRequest {
        /// Error message
        message: String,
        /// Parameter that caused the error
        param: Option<String>,
    },

    /// Model already exists (when creating/copying).
    ModelAlreadyExists {
        /// Model identifier
        model: String,
    },

    /// API error from Ollama.
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
        last_error: Box<OllamaError>,
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

    /// Base64 encoding/decoding error.
    Base64 {
        /// Error message
        message: String,
    },

    /// Image processing error.
    Image {
        /// Error message
        message: String,
    },
}

impl fmt::Display for OllamaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OllamaError::Connection { message } => {
                write!(f, "Connection failed (is Ollama running?): {message}")
            }
            OllamaError::ModelNotFound { model } => {
                write!(f, "Model not found: {model}")
            }
            OllamaError::InvalidRequest { message, param } => {
                if let Some(p) = param {
                    write!(f, "Invalid request ({p}): {message}")
                } else {
                    write!(f, "Invalid request: {message}")
                }
            }
            OllamaError::ModelAlreadyExists { model } => {
                write!(f, "Model already exists: {model}")
            }
            OllamaError::Api { status, message } => {
                write!(f, "API error ({status}): {message}")
            }
            OllamaError::Http { source } => {
                write!(f, "HTTP error: {source}")
            }
            OllamaError::Json { source } => {
                write!(f, "JSON error: {source}")
            }
            OllamaError::Stream { message } => {
                write!(f, "Stream error: {message}")
            }
            OllamaError::Config { message } => {
                write!(f, "Configuration error: {message}")
            }
            OllamaError::Timeout { operation } => {
                write!(f, "Timeout during {operation}")
            }
            OllamaError::RetryExhausted {
                attempts,
                last_error,
            } => {
                write!(f, "Retry exhausted after {attempts} attempts: {last_error}")
            }
            OllamaError::NotFound { resource, id } => {
                write!(f, "{resource} not found: {id}")
            }
            OllamaError::Internal { message } => {
                write!(f, "Internal error: {message}")
            }
            OllamaError::Io { source } => {
                write!(f, "I/O error: {source}")
            }
            OllamaError::Base64 { message } => {
                write!(f, "Base64 error: {message}")
            }
            OllamaError::Image { message } => {
                write!(f, "Image error: {message}")
            }
        }
    }
}

impl std::error::Error for OllamaError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            OllamaError::Http { source } => Some(source),
            OllamaError::Json { source } => Some(source),
            OllamaError::RetryExhausted { last_error, .. } => Some(last_error),
            OllamaError::Io { source } => Some(source),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for OllamaError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            OllamaError::Timeout {
                operation: "HTTP request".to_string(),
            }
        } else if err.is_connect() {
            OllamaError::Connection {
                message: err.to_string(),
            }
        } else {
            OllamaError::Http { source: err }
        }
    }
}

impl From<serde_json::Error> for OllamaError {
    fn from(err: serde_json::Error) -> Self {
        OllamaError::Json { source: err }
    }
}

impl From<std::io::Error> for OllamaError {
    fn from(err: std::io::Error) -> Self {
        OllamaError::Io { source: err }
    }
}

impl OllamaError {
    /// Check if this error is retryable.
    pub fn is_retryable(&self) -> bool {
        match self {
            OllamaError::Connection { .. }
            | OllamaError::Http { .. }
            | OllamaError::Timeout { .. } => true,
            OllamaError::Api { status, .. } => {
                status.is_server_error() || *status == reqwest::StatusCode::TOO_MANY_REQUESTS
            }
            _ => false,
        }
    }

    /// Check if this is a connection error.
    pub fn is_connection_error(&self) -> bool {
        matches!(self, OllamaError::Connection { .. })
    }

    /// Check if this is a model not found error.
    pub fn is_model_not_found(&self) -> bool {
        matches!(self, OllamaError::ModelNotFound { .. })
    }

    /// Create an error from an HTTP response.
    pub(crate) async fn from_response(response: reqwest::Response) -> Self {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        // Try to parse Ollama's error format
        if let Ok(error_json) = serde_json::from_str::<serde_json::Value>(&body) {
            if let Some(error_obj) = error_json.get("error") {
                let message = error_obj
                    .as_str()
                    .or_else(|| error_obj.get("message").and_then(|v| v.as_str()))
                    .unwrap_or(&body)
                    .to_string();

                // Map specific error patterns
                if message.contains("not found") || message.contains("does not exist") {
                    let model = message
                        .split_whitespace()
                        .next()
                        .unwrap_or("unknown")
                        .to_string();
                    return OllamaError::ModelNotFound { model };
                }

                if message.contains("already exists") {
                    let model = message
                        .split_whitespace()
                        .next()
                        .unwrap_or("unknown")
                        .to_string();
                    return OllamaError::ModelAlreadyExists { model };
                }

                return OllamaError::Api { status, message };
            }
        }

        // Fallback for non-JSON responses
        OllamaError::Api {
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
        let err = OllamaError::Connection {
            message: "Connection refused".to_string(),
        };
        assert!(err.to_string().contains("Connection failed"));
    }

    #[test]
    fn test_is_retryable() {
        let connection = OllamaError::Connection {
            message: "test".to_string(),
        };
        assert!(connection.is_retryable());

        let not_found = OllamaError::ModelNotFound {
            model: "test".to_string(),
        };
        assert!(!not_found.is_retryable());
    }

    #[test]
    fn test_is_connection_error() {
        let connection = OllamaError::Connection {
            message: "test".to_string(),
        };
        assert!(connection.is_connection_error());

        let other = OllamaError::Internal {
            message: "test".to_string(),
        };
        assert!(!other.is_connection_error());
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: OllamaError = io_err.into();
        assert!(matches!(err, OllamaError::Io { .. }));
    }

    #[test]
    fn test_json_error_conversion() {
        // Create a JSON error by parsing invalid JSON
        let result: std::result::Result<serde_json::Value, serde_json::Error> =
            serde_json::from_str("invalid json");
        let json_err = result.unwrap_err();
        let err: OllamaError = json_err.into();
        assert!(matches!(err, OllamaError::Json { .. }));
    }
}
