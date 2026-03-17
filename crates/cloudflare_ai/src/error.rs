//! Error types for the Cloudflare AI SDK.

use std::fmt;

/// Result type alias for Cloudflare AI operations.
pub type Result<T> = std::result::Result<T, CloudflareAiError>;

/// Errors that can occur when using the Cloudflare Workers AI API.
#[derive(Debug)]
pub enum CloudflareAiError {
    /// Authentication failed (invalid API token).
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

    /// Model not found or doesn't exist.
    ModelNotFound {
        /// Model identifier
        model: String,
    },

    /// API error from Cloudflare.
    Api {
        /// HTTP status code
        status: reqwest::StatusCode,
        /// Error code from Cloudflare
        code: Option<u32>,
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
        last_error: Box<CloudflareAiError>,
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

    /// I/O error (for file operations).
    Io {
        /// Source error
        source: std::io::Error,
    },

    /// Image processing error.
    Image {
        /// Error message
        message: String,
    },

    /// Audio processing error.
    Audio {
        /// Error message
        message: String,
    },
}

impl fmt::Display for CloudflareAiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CloudflareAiError::Authentication { message } => {
                write!(f, "Authentication failed: {message}")
            }
            CloudflareAiError::RateLimit { retry_after, message } => {
                if let Some(duration) = retry_after {
                    write!(f, "Rate limit exceeded (retry after {duration:?}): {message}")
                } else {
                    write!(f, "Rate limit exceeded: {message}")
                }
            }
            CloudflareAiError::InvalidRequest { message, param } => {
                if let Some(p) = param {
                    write!(f, "Invalid request ({p}): {message}")
                } else {
                    write!(f, "Invalid request: {message}")
                }
            }
            CloudflareAiError::ModelNotFound { model } => {
                write!(f, "Model not found: {model}")
            }
            CloudflareAiError::Api { status, code, message } => {
                if let Some(c) = code {
                    write!(f, "API error ({status} - code {c}): {message}")
                } else {
                    write!(f, "API error ({status}): {message}")
                }
            }
            CloudflareAiError::Http { source } => {
                write!(f, "HTTP error: {source}")
            }
            CloudflareAiError::Json { source } => {
                write!(f, "JSON error: {source}")
            }
            CloudflareAiError::Stream { message } => {
                write!(f, "Stream error: {message}")
            }
            CloudflareAiError::Config { message } => {
                write!(f, "Configuration error: {message}")
            }
            CloudflareAiError::Timeout { operation } => {
                write!(f, "Timeout during {operation}")
            }
            CloudflareAiError::RetryExhausted { attempts, last_error } => {
                write!(f, "Retry exhausted after {attempts} attempts: {last_error}")
            }
            CloudflareAiError::NotFound { resource, id } => {
                write!(f, "{resource} not found: {id}")
            }
            CloudflareAiError::Internal { message } => {
                write!(f, "Internal error: {message}")
            }
            CloudflareAiError::Io { source } => {
                write!(f, "I/O error: {source}")
            }
            CloudflareAiError::Image { message } => {
                write!(f, "Image error: {message}")
            }
            CloudflareAiError::Audio { message } => {
                write!(f, "Audio error: {message}")
            }
        }
    }
}

impl std::error::Error for CloudflareAiError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CloudflareAiError::Http { source } => Some(source),
            CloudflareAiError::Json { source } => Some(source),
            CloudflareAiError::RetryExhausted { last_error, .. } => Some(last_error),
            CloudflareAiError::Io { source } => Some(source),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for CloudflareAiError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            CloudflareAiError::Timeout {
                operation: "HTTP request".to_string(),
            }
        } else {
            CloudflareAiError::Http { source: err }
        }
    }
}

impl From<serde_json::Error> for CloudflareAiError {
    fn from(err: serde_json::Error) -> Self {
        CloudflareAiError::Json { source: err }
    }
}

impl From<std::io::Error> for CloudflareAiError {
    fn from(err: std::io::Error) -> Self {
        CloudflareAiError::Io { source: err }
    }
}

impl CloudflareAiError {
    /// Check if this error is retryable.
    pub fn is_retryable(&self) -> bool {
        match self {
            CloudflareAiError::RateLimit { .. } | CloudflareAiError::Http { .. } => true,
            CloudflareAiError::Api { status, .. } => {
                status.is_server_error() || *status == reqwest::StatusCode::TOO_MANY_REQUESTS
            }
            _ => false,
        }
    }

    /// Check if this is a rate limit error.
    pub fn is_rate_limit(&self) -> bool {
        matches!(self, CloudflareAiError::RateLimit { .. })
    }

    /// Check if this is an authentication error.
    pub fn is_auth_error(&self) -> bool {
        matches!(self, CloudflareAiError::Authentication { .. })
    }

    /// Get the retry-after duration if available.
    pub fn retry_after(&self) -> Option<std::time::Duration> {
        match self {
            CloudflareAiError::RateLimit { retry_after, .. } => *retry_after,
            _ => None,
        }
    }

    /// Create an error from an HTTP response.
    pub(crate) async fn from_response(response: reqwest::Response) -> Self {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        // Try to parse Cloudflare's error format
        if let Ok(error_json) = serde_json::from_str::<serde_json::Value>(&body) {
            if let Some(errors) = error_json.get("errors").and_then(|v| v.as_array()) {
                if let Some(first_error) = errors.first() {
                    let code = first_error.get("code").and_then(|v| v.as_u64()).map(|c| c as u32);
                    let message = first_error
                        .get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&body)
                        .to_string();

                    // Map specific Cloudflare error codes
                    match code {
                        Some(10000) => return CloudflareAiError::Authentication { message },
                        Some(10001) => {
                            return CloudflareAiError::RateLimit {
                                retry_after: None,
                                message,
                            }
                        }
                        _ => {
                            if status == reqwest::StatusCode::UNAUTHORIZED {
                                return CloudflareAiError::Authentication { message };
                            }
                            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                                return CloudflareAiError::RateLimit {
                                    retry_after: None,
                                    message,
                                };
                            }
                        }
                    }

                    return CloudflareAiError::Api {
                        status,
                        code,
                        message,
                    };
                }
            }

            // Try alternative error format
            if let Some(message) = error_json.get("message").and_then(|v| v.as_str()) {
                if status == reqwest::StatusCode::UNAUTHORIZED {
                    return CloudflareAiError::Authentication {
                        message: message.to_string(),
                    };
                }
                return CloudflareAiError::Api {
                    status,
                    code: None,
                    message: message.to_string(),
                };
            }
        }

        // Fallback for non-JSON responses
        CloudflareAiError::Api {
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
        let err = CloudflareAiError::Authentication {
            message: "Invalid API token".to_string(),
        };
        assert!(err.to_string().contains("Authentication failed"));
    }

    #[test]
    fn test_is_retryable() {
        let rate_limit = CloudflareAiError::RateLimit {
            retry_after: None,
            message: "Too many requests".to_string(),
        };
        assert!(rate_limit.is_retryable());

        let auth = CloudflareAiError::Authentication {
            message: "Invalid token".to_string(),
        };
        assert!(!auth.is_retryable());
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let cf_err: CloudflareAiError = io_err.into();
        assert!(matches!(cf_err, CloudflareAiError::Io { .. }));
    }

    #[test]
    fn test_model_not_found() {
        let err = CloudflareAiError::ModelNotFound {
            model: "@cf/meta/unknown-model".to_string(),
        };
        assert!(err.to_string().contains("@cf/meta/unknown-model"));
    }
}
