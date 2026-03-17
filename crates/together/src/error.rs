//! Error types for the Together AI SDK.

use std::fmt;

/// Result type alias for Together AI operations.
pub type Result<T> = std::result::Result<T, TogetherError>;

/// Errors that can occur when using the Together AI API.
#[derive(Debug)]
pub enum TogetherError {
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
}

impl fmt::Display for TogetherError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TogetherError::Authentication { message } => {
                write!(f, "Authentication failed: {message}")
            }
            TogetherError::RateLimit { retry_after, message } => {
                if let Some(duration) = retry_after {
                    write!(f, "Rate limit exceeded (retry after {duration:?}): {message}")
                } else {
                    write!(f, "Rate limit exceeded: {message}")
                }
            }
            TogetherError::InvalidRequest { message } => {
                write!(f, "Invalid request: {message}")
            }
            TogetherError::ModelNotFound { model } => {
                write!(f, "Model not found: {model}")
            }
            TogetherError::Api { status, message } => {
                write!(f, "API error ({status}): {message}")
            }
            TogetherError::Http { source } => {
                write!(f, "HTTP error: {source}")
            }
            TogetherError::Json { source } => {
                write!(f, "JSON error: {source}")
            }
            TogetherError::Stream { message } => {
                write!(f, "Stream error: {message}")
            }
            TogetherError::Config { message } => {
                write!(f, "Configuration error: {message}")
            }
            TogetherError::Timeout { operation } => {
                write!(f, "Timeout during {operation}")
            }
            TogetherError::Internal { message } => {
                write!(f, "Internal error: {message}")
            }
            TogetherError::Io { source } => {
                write!(f, "I/O error: {source}")
            }
        }
    }
}

impl std::error::Error for TogetherError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TogetherError::Http { source } => Some(source),
            TogetherError::Json { source } => Some(source),
            TogetherError::Io { source } => Some(source),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for TogetherError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            TogetherError::Timeout {
                operation: "HTTP request".to_string(),
            }
        } else {
            TogetherError::Http { source: err }
        }
    }
}

impl From<serde_json::Error> for TogetherError {
    fn from(err: serde_json::Error) -> Self {
        TogetherError::Json { source: err }
    }
}

impl From<std::io::Error> for TogetherError {
    fn from(err: std::io::Error) -> Self {
        TogetherError::Io { source: err }
    }
}

impl TogetherError {
    /// Check if this error is retryable.
    pub fn is_retryable(&self) -> bool {
        match self {
            TogetherError::RateLimit { .. } | TogetherError::Http { .. } => true,
            TogetherError::Api { status, .. } => {
                status.is_server_error() || *status == reqwest::StatusCode::TOO_MANY_REQUESTS
            }
            _ => false,
        }
    }

    /// Create an error from an HTTP response.
    pub(crate) async fn from_response(response: reqwest::Response) -> Self {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        // Try to parse Together AI's error format
        if let Ok(error_json) = serde_json::from_str::<serde_json::Value>(&body) {
            if let Some(error_obj) = error_json.get("error") {
                let message = error_obj
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&body)
                    .to_string();

                if status == reqwest::StatusCode::UNAUTHORIZED {
                    return TogetherError::Authentication { message };
                }

                if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                    return TogetherError::RateLimit {
                        retry_after: None,
                        message,
                    };
                }

                if status == reqwest::StatusCode::NOT_FOUND {
                    return TogetherError::ModelNotFound { model: message };
                }

                if status == reqwest::StatusCode::BAD_REQUEST {
                    return TogetherError::InvalidRequest { message };
                }

                return TogetherError::Api { status, message };
            }

            // Alternative error format (message at root)
            if let Some(message) = error_json
                .get("message")
                .and_then(|v| v.as_str())
            {
                if status == reqwest::StatusCode::UNAUTHORIZED {
                    return TogetherError::Authentication {
                        message: message.to_string(),
                    };
                }

                if status == reqwest::StatusCode::BAD_REQUEST {
                    return TogetherError::InvalidRequest {
                        message: message.to_string(),
                    };
                }

                return TogetherError::Api {
                    status,
                    message: message.to_string(),
                };
            }
        }

        TogetherError::Api {
            status,
            message: body,
        }
    }
}
