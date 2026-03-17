//! Error types for the Fireworks AI SDK.

use std::fmt;

/// Result type alias for Fireworks AI operations.
pub type Result<T> = std::result::Result<T, FireworksError>;

/// Errors that can occur when using the Fireworks AI API.
#[derive(Debug)]
pub enum FireworksError {
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

    /// File I/O error.
    Io {
        /// Source error
        source: std::io::Error,
    },

    /// Retry exhausted.
    RetryExhausted {
        /// Number of attempts made
        attempts: u32,
        /// Last error
        last_error: Box<FireworksError>,
    },

    /// Not found error.
    NotFound {
        /// Resource type
        resource: String,
        /// Resource ID
        id: String,
    },
}

impl fmt::Display for FireworksError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FireworksError::Authentication { message } => {
                write!(f, "Authentication failed: {message}")
            }
            FireworksError::RateLimit {
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
            FireworksError::InvalidRequest { message } => {
                write!(f, "Invalid request: {message}")
            }
            FireworksError::ModelNotFound { model } => {
                write!(f, "Model not found: {model}")
            }
            FireworksError::Api { status, message } => {
                write!(f, "API error ({status}): {message}")
            }
            FireworksError::Http { source } => {
                write!(f, "HTTP error: {source}")
            }
            FireworksError::Json { source } => {
                write!(f, "JSON error: {source}")
            }
            FireworksError::Stream { message } => {
                write!(f, "Stream error: {message}")
            }
            FireworksError::Config { message } => {
                write!(f, "Configuration error: {message}")
            }
            FireworksError::Timeout { operation } => {
                write!(f, "Timeout during {operation}")
            }
            FireworksError::Internal { message } => {
                write!(f, "Internal error: {message}")
            }
            FireworksError::Io { source } => {
                write!(f, "I/O error: {source}")
            }
            FireworksError::RetryExhausted {
                attempts,
                last_error,
            } => {
                write!(f, "Retry exhausted after {attempts} attempts: {last_error}")
            }
            FireworksError::NotFound { resource, id } => {
                write!(f, "{resource} not found: {id}")
            }
        }
    }
}

impl std::error::Error for FireworksError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FireworksError::Http { source } => Some(source),
            FireworksError::Json { source } => Some(source),
            FireworksError::Io { source } => Some(source),
            FireworksError::RetryExhausted { last_error, .. } => Some(last_error),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for FireworksError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            FireworksError::Timeout {
                operation: "HTTP request".to_string(),
            }
        } else {
            FireworksError::Http { source: err }
        }
    }
}

impl From<serde_json::Error> for FireworksError {
    fn from(err: serde_json::Error) -> Self {
        FireworksError::Json { source: err }
    }
}

impl From<std::io::Error> for FireworksError {
    fn from(err: std::io::Error) -> Self {
        FireworksError::Io { source: err }
    }
}

impl FireworksError {
    /// Check if this error is retryable.
    pub fn is_retryable(&self) -> bool {
        match self {
            FireworksError::RateLimit { .. } | FireworksError::Http { .. } => true,
            FireworksError::Api { status, .. } => {
                status.is_server_error() || *status == reqwest::StatusCode::TOO_MANY_REQUESTS
            }
            FireworksError::RetryExhausted { .. } => false,
            _ => false,
        }
    }

    /// Get the retry after duration if available.
    pub fn retry_after(&self) -> Option<std::time::Duration> {
        match self {
            FireworksError::RateLimit { retry_after, .. } => *retry_after,
            _ => None,
        }
    }

    /// Create an error from an HTTP response.
    pub(crate) async fn from_response(response: reqwest::Response) -> Self {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        // Try to parse Fireworks AI's error format
        if let Ok(error_json) = serde_json::from_str::<serde_json::Value>(&body) {
            // Standard error format
            if let Some(error_obj) = error_json.get("error") {
                let message = error_obj
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&body)
                    .to_string();

                if status == reqwest::StatusCode::UNAUTHORIZED {
                    return FireworksError::Authentication { message };
                }

                if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                    return FireworksError::RateLimit {
                        retry_after: None,
                        message,
                    };
                }

                if status == reqwest::StatusCode::NOT_FOUND {
                    return FireworksError::ModelNotFound { model: message };
                }

                if status == reqwest::StatusCode::BAD_REQUEST {
                    return FireworksError::InvalidRequest { message };
                }

                return FireworksError::Api { status, message };
            }

            // Alternative error format (message at root)
            if let Some(message) = error_json.get("message").and_then(|v| v.as_str()) {
                if status == reqwest::StatusCode::UNAUTHORIZED {
                    return FireworksError::Authentication {
                        message: message.to_string(),
                    };
                }

                if status == reqwest::StatusCode::BAD_REQUEST {
                    return FireworksError::InvalidRequest {
                        message: message.to_string(),
                    };
                }

                return FireworksError::Api {
                    status,
                    message: message.to_string(),
                };
            }
        }

        FireworksError::Api {
            status,
            message: body,
        }
    }
}
