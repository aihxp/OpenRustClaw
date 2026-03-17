//! Error types for the OpenRouter SDK.

use std::fmt;

/// Result type alias for OpenRouter operations.
pub type Result<T> = std::result::Result<T, OpenRouterError>;

/// Errors that can occur when using the OpenRouter API.
#[derive(Debug)]
pub enum OpenRouterError {
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

    /// Provider error.
    Provider {
        /// Provider name
        provider: String,
        /// Error message
        message: String,
    },

    /// All providers in fallback chain failed.
    FallbackExhausted {
        /// Errors from each provider
        errors: Vec<(String, Box<OpenRouterError>)>,
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
}

impl fmt::Display for OpenRouterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpenRouterError::Authentication { message } => {
                write!(f, "Authentication failed: {message}")
            }
            OpenRouterError::RateLimit { retry_after, message } => {
                if let Some(duration) = retry_after {
                    write!(f, "Rate limit exceeded (retry after {duration:?}): {message}")
                } else {
                    write!(f, "Rate limit exceeded: {message}")
                }
            }
            OpenRouterError::InvalidRequest { message } => {
                write!(f, "Invalid request: {message}")
            }
            OpenRouterError::ModelNotFound { model } => {
                write!(f, "Model not found: {model}")
            }
            OpenRouterError::Provider { provider, message } => {
                write!(f, "Provider error ({provider}): {message}")
            }
            OpenRouterError::FallbackExhausted { errors } => {
                write!(f, "All providers failed: {:?}", errors.iter().map(|(p, _)| p).collect::<Vec<_>>())
            }
            OpenRouterError::Api { status, message } => {
                write!(f, "API error ({status}): {message}")
            }
            OpenRouterError::Http { source } => {
                write!(f, "HTTP error: {source}")
            }
            OpenRouterError::Json { source } => {
                write!(f, "JSON error: {source}")
            }
            OpenRouterError::Stream { message } => {
                write!(f, "Stream error: {message}")
            }
            OpenRouterError::Config { message } => {
                write!(f, "Configuration error: {message}")
            }
            OpenRouterError::Timeout { operation } => {
                write!(f, "Timeout during {operation}")
            }
            OpenRouterError::Internal { message } => {
                write!(f, "Internal error: {message}")
            }
        }
    }
}

impl std::error::Error for OpenRouterError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            OpenRouterError::Http { source } => Some(source),
            OpenRouterError::Json { source } => Some(source),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for OpenRouterError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            OpenRouterError::Timeout {
                operation: "HTTP request".to_string(),
            }
        } else {
            OpenRouterError::Http { source: err }
        }
    }
}

impl From<serde_json::Error> for OpenRouterError {
    fn from(err: serde_json::Error) -> Self {
        OpenRouterError::Json { source: err }
    }
}

impl OpenRouterError {
    /// Check if this error is retryable.
    pub fn is_retryable(&self) -> bool {
        match self {
            OpenRouterError::RateLimit { .. } 
                | OpenRouterError::Http { .. } 
                | OpenRouterError::Provider { .. } => true,
            OpenRouterError::Api { status, .. } => {
                status.is_server_error() || *status == reqwest::StatusCode::TOO_MANY_REQUESTS
            }
            _ => false,
        }
    }

    /// Create an error from an HTTP response.
    pub(crate) async fn from_response(response: reqwest::Response) -> Self {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        // Try to parse OpenRouter's error format
        if let Ok(error_json) = serde_json::from_str::<serde_json::Value>(&body) {
            if let Some(error_obj) = error_json.get("error") {
                let message = error_obj
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&body)
                    .to_string();

                if status == reqwest::StatusCode::UNAUTHORIZED {
                    return OpenRouterError::Authentication { message };
                }

                if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                    return OpenRouterError::RateLimit {
                        retry_after: None,
                        message,
                    };
                }

                if status == reqwest::StatusCode::NOT_FOUND {
                    return OpenRouterError::ModelNotFound { model: message };
                }

                return OpenRouterError::Api { status, message };
            }
        }

        OpenRouterError::Api {
            status,
            message: body,
        }
    }
}
