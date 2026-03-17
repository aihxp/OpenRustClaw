//! Error types for the vLLM SDK.

use std::fmt;

/// Result type alias for vLLM operations.
pub type Result<T> = std::result::Result<T, VllmError>;

/// Errors that can occur when using the vLLM API.
#[derive(Debug)]
pub enum VllmError {
    /// Authentication failed.
    Authentication {
        /// Error message
        message: String,
    },

    /// Rate limit exceeded.
    RateLimit {
        /// Retry after duration
        retry_after: Option<std::time::Duration>,
        /// Error message
        message: String,
    },

    /// Invalid request.
    InvalidRequest {
        /// Error message
        message: String,
    },

    /// Model not found.
    ModelNotFound {
        /// Model identifier
        model: String,
    },

    /// API error.
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

    /// JSON error.
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

    /// Server overload - vLLM specific.
    ServerOverload {
        /// Error message
        message: String,
    },

    /// Health check failed.
    Unhealthy {
        /// Error message
        message: String,
    },
}

impl fmt::Display for VllmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VllmError::Authentication { message } => {
                write!(f, "Authentication failed: {message}")
            }
            VllmError::RateLimit { retry_after, message } => {
                if let Some(duration) = retry_after {
                    write!(f, "Rate limit exceeded (retry after {duration:?}): {message}")
                } else {
                    write!(f, "Rate limit exceeded: {message}")
                }
            }
            VllmError::InvalidRequest { message } => {
                write!(f, "Invalid request: {message}")
            }
            VllmError::ModelNotFound { model } => {
                write!(f, "Model not found: {model}")
            }
            VllmError::Api { status, message } => {
                write!(f, "API error ({status}): {message}")
            }
            VllmError::Http { source } => {
                write!(f, "HTTP error: {source}")
            }
            VllmError::Json { source } => {
                write!(f, "JSON error: {source}")
            }
            VllmError::Stream { message } => {
                write!(f, "Stream error: {message}")
            }
            VllmError::Config { message } => {
                write!(f, "Configuration error: {message}")
            }
            VllmError::Timeout { operation } => {
                write!(f, "Timeout during {operation}")
            }
            VllmError::Internal { message } => {
                write!(f, "Internal error: {message}")
            }
            VllmError::Io { source } => {
                write!(f, "I/O error: {source}")
            }
            VllmError::ServerOverload { message } => {
                write!(f, "Server overloaded: {message}")
            }
            VllmError::Unhealthy { message } => {
                write!(f, "Health check failed: {message}")
            }
        }
    }
}

impl std::error::Error for VllmError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            VllmError::Http { source } => Some(source),
            VllmError::Json { source } => Some(source),
            VllmError::Io { source } => Some(source),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for VllmError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            VllmError::Timeout {
                operation: "HTTP request".to_string(),
            }
        } else {
            VllmError::Http { source: err }
        }
    }
}

impl From<serde_json::Error> for VllmError {
    fn from(err: serde_json::Error) -> Self {
        VllmError::Json { source: err }
    }
}

impl From<std::io::Error> for VllmError {
    fn from(err: std::io::Error) -> Self {
        VllmError::Io { source: err }
    }
}

impl VllmError {
    /// Check if this error is retryable.
    pub fn is_retryable(&self) -> bool {
        match self {
            VllmError::RateLimit { .. } | VllmError::Http { .. } => true,
            VllmError::Api { status, .. } => {
                status.is_server_error() || *status == reqwest::StatusCode::TOO_MANY_REQUESTS
            }
            VllmError::ServerOverload { .. } => true,
            _ => false,
        }
    }

    /// Create an error from an HTTP response.
    pub(crate) async fn from_response(response: reqwest::Response) -> Self {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        // Try to parse OpenAI/vLLM error format
        if let Ok(error_json) = serde_json::from_str::<serde_json::Value>(&body) {
            if let Some(error_obj) = error_json.get("error") {
                let message = error_obj
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&body)
                    .to_string();

                if status == reqwest::StatusCode::UNAUTHORIZED {
                    return VllmError::Authentication { message };
                }

                if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                    return VllmError::RateLimit {
                        retry_after: None,
                        message,
                    };
                }

                if status == reqwest::StatusCode::NOT_FOUND {
                    return VllmError::ModelNotFound { model: message };
                }

                if status == reqwest::StatusCode::BAD_REQUEST {
                    return VllmError::InvalidRequest { message };
                }

                if status == reqwest::StatusCode::SERVICE_UNAVAILABLE {
                    return VllmError::ServerOverload { message };
                }

                return VllmError::Api { status, message };
            }

            // Alternative error format (message at root)
            if let Some(message) = error_json.get("message").and_then(|v| v.as_str()) {
                if status == reqwest::StatusCode::UNAUTHORIZED {
                    return VllmError::Authentication {
                        message: message.to_string(),
                    };
                }

                if status == reqwest::StatusCode::BAD_REQUEST {
                    return VllmError::InvalidRequest {
                        message: message.to_string(),
                    };
                }

                return VllmError::Api {
                    status,
                    message: message.to_string(),
                };
            }
        }

        VllmError::Api {
            status,
            message: body,
        }
    }
}
