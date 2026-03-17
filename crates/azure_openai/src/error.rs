//! Error types for the Azure OpenAI SDK.

use std::fmt;

/// Result type alias for Azure OpenAI operations.
pub type Result<T> = std::result::Result<T, AzureOpenAIError>;

/// Errors that can occur when using the Azure OpenAI API.
#[derive(Debug)]
pub enum AzureOpenAIError {
    /// Authentication failed (invalid API key or Azure AD token).
    Authentication {
        /// Error message
        message: String,
    },

    /// Azure AD token acquisition failed.
    TokenAcquisition {
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

    /// Deployment not found.
    DeploymentNotFound {
        /// Deployment name
        deployment: String,
    },

    /// Content filtered by Azure Content Safety.
    ContentFiltered {
        /// Content filter results
        filter_results: ContentFilterResults,
        /// Error message
        message: String,
    },

    /// API error from Azure OpenAI.
    Api {
        /// HTTP status code
        status: reqwest::StatusCode,
        /// Error code from Azure
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
        last_error: Box<AzureOpenAIError>,
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
}

/// Content filter results.
#[derive(Debug, Clone, Default)]
pub struct ContentFilterResults {
    /// Hate speech filter result.
    pub hate: Option<ContentFilterResult>,
    /// Self-harm filter result.
    pub self_harm: Option<ContentFilterResult>,
    /// Sexual content filter result.
    pub sexual: Option<ContentFilterResult>,
    /// Violence filter result.
    pub violence: Option<ContentFilterResult>,
    /// Profanity filter result.
    pub profanity: Option<ContentFilterResult>,
    /// Custom blocklist filter result.
    pub custom_blocklists: Option<ContentFilterResult>,
    /// Error if content filter failed.
    pub error: Option<String>,
}

/// Individual content filter result.
#[derive(Debug, Clone)]
pub struct ContentFilterResult {
    /// Whether the content was filtered.
    pub filtered: bool,
    /// Severity level if detected.
    pub severity: Option<ContentFilterSeverity>,
}

/// Content filter severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentFilterSeverity {
    /// Safe content.
    Safe,
    /// Low severity.
    Low,
    /// Medium severity.
    Medium,
    /// High severity.
    High,
}

impl fmt::Display for AzureOpenAIError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AzureOpenAIError::Authentication { message } => {
                write!(f, "Authentication failed: {message}")
            }
            AzureOpenAIError::TokenAcquisition { message } => {
                write!(f, "Azure AD token acquisition failed: {message}")
            }
            AzureOpenAIError::RateLimit {
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
            AzureOpenAIError::InvalidRequest { message, param } => {
                if let Some(p) = param {
                    write!(f, "Invalid request ({p}): {message}")
                } else {
                    write!(f, "Invalid request: {message}")
                }
            }
            AzureOpenAIError::ContextLength {
                tokens_used,
                max_tokens,
            } => {
                write!(
                    f,
                    "Context length exceeded: {tokens_used} tokens used, {max_tokens} max"
                )
            }
            AzureOpenAIError::ModelNotFound { model } => {
                write!(f, "Model not found: {model}")
            }
            AzureOpenAIError::DeploymentNotFound { deployment } => {
                write!(f, "Deployment not found: {deployment}")
            }
            AzureOpenAIError::ContentFiltered { message, .. } => {
                write!(f, "Content filtered: {message}")
            }
            AzureOpenAIError::Api {
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
            AzureOpenAIError::Http { source } => {
                write!(f, "HTTP error: {source}")
            }
            AzureOpenAIError::Json { source } => {
                write!(f, "JSON error: {source}")
            }
            AzureOpenAIError::Stream { message } => {
                write!(f, "Stream error: {message}")
            }
            AzureOpenAIError::Config { message } => {
                write!(f, "Configuration error: {message}")
            }
            AzureOpenAIError::Timeout { operation } => {
                write!(f, "Timeout during {operation}")
            }
            AzureOpenAIError::RetryExhausted {
                attempts,
                last_error,
            } => {
                write!(f, "Retry exhausted after {attempts} attempts: {last_error}")
            }
            AzureOpenAIError::NotFound { resource, id } => {
                write!(f, "{resource} not found: {id}")
            }
            AzureOpenAIError::Internal { message } => {
                write!(f, "Internal error: {message}")
            }
        }
    }
}

impl std::error::Error for AzureOpenAIError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AzureOpenAIError::Http { source } => Some(source),
            AzureOpenAIError::Json { source } => Some(source),
            AzureOpenAIError::RetryExhausted { last_error, .. } => Some(last_error),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for AzureOpenAIError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            AzureOpenAIError::Timeout {
                operation: "HTTP request".to_string(),
            }
        } else {
            AzureOpenAIError::Http { source: err }
        }
    }
}

impl From<serde_json::Error> for AzureOpenAIError {
    fn from(err: serde_json::Error) -> Self {
        AzureOpenAIError::Json { source: err }
    }
}

impl AzureOpenAIError {
    /// Check if this error is retryable.
    pub fn is_retryable(&self) -> bool {
        match self {
            AzureOpenAIError::RateLimit { .. } | AzureOpenAIError::Http { .. } => true,
            AzureOpenAIError::Api { status, .. } => {
                status.is_server_error() || *status == reqwest::StatusCode::TOO_MANY_REQUESTS
            }
            _ => false,
        }
    }

    /// Check if this is a rate limit error.
    pub fn is_rate_limit(&self) -> bool {
        matches!(self, AzureOpenAIError::RateLimit { .. })
    }

    /// Check if this is an authentication error.
    pub fn is_auth_error(&self) -> bool {
        matches!(
            self,
            AzureOpenAIError::Authentication { .. } | AzureOpenAIError::TokenAcquisition { .. }
        )
    }

    /// Check if this is a content filtering error.
    pub fn is_content_filtered(&self) -> bool {
        matches!(self, AzureOpenAIError::ContentFiltered { .. })
    }

    /// Get the retry-after duration if available.
    pub fn retry_after(&self) -> Option<std::time::Duration> {
        match self {
            AzureOpenAIError::RateLimit { retry_after, .. } => *retry_after,
            _ => None,
        }
    }

    /// Create an error from an HTTP response.
    pub(crate) async fn from_response(response: reqwest::Response) -> Self {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        // Try to parse Azure OpenAI's error format
        if let Ok(error_json) = serde_json::from_str::<serde_json::Value>(&body) {
            // Check for Azure Content Safety filtering
            if let Some(content_filter) = error_json.get("content_filter_result") {
                return Self::parse_content_filter_error(content_filter, &body);
            }

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
                    Some("invalid_api_key")
                    | Some("unauthorized")
                    | Some("AuthenticationError") => {
                        return AzureOpenAIError::Authentication { message };
                    }
                    Some("rate_limit_exceeded") | Some("RateLimitError") => {
                        return AzureOpenAIError::RateLimit {
                            retry_after: None,
                            message,
                        };
                    }
                    Some("insufficient_quota") => {
                        return AzureOpenAIError::RateLimit {
                            retry_after: None,
                            message,
                        };
                    }
                    Some("invalid_request_error") | Some("InvalidRequest") => {
                        return AzureOpenAIError::InvalidRequest { message, param };
                    }
                    Some("context_length_exceeded") => {
                        return AzureOpenAIError::ContextLength {
                            tokens_used: 0,
                            max_tokens: 0,
                        };
                    }
                    Some("model_not_found") | Some("DeploymentNotFound") => {
                        return AzureOpenAIError::DeploymentNotFound {
                            deployment: message,
                        };
                    }
                    Some("content_filter") | Some("ContentFiltered") => {
                        return AzureOpenAIError::ContentFiltered {
                            filter_results: ContentFilterResults::default(),
                            message,
                        };
                    }
                    _ => {}
                }

                return AzureOpenAIError::Api {
                    status,
                    code,
                    message,
                };
            }
        }

        // Fallback for non-JSON responses
        AzureOpenAIError::Api {
            status,
            code: None,
            message: body,
        }
    }

    fn parse_content_filter_error(
        content_filter: &serde_json::Value,
        body: &str,
    ) -> AzureOpenAIError {
        let mut results = ContentFilterResults::default();

        if let Some(hate) = content_filter.get("hate") {
            results.hate = Some(ContentFilterResult {
                filtered: hate
                    .get("filtered")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                severity: hate
                    .get("severity")
                    .and_then(|v| v.as_str())
                    .and_then(parse_severity),
            });
        }

        if let Some(self_harm) = content_filter.get("self_harm") {
            results.self_harm = Some(ContentFilterResult {
                filtered: self_harm
                    .get("filtered")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                severity: self_harm
                    .get("severity")
                    .and_then(|v| v.as_str())
                    .and_then(parse_severity),
            });
        }

        if let Some(sexual) = content_filter.get("sexual") {
            results.sexual = Some(ContentFilterResult {
                filtered: sexual
                    .get("filtered")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                severity: sexual
                    .get("severity")
                    .and_then(|v| v.as_str())
                    .and_then(parse_severity),
            });
        }

        if let Some(violence) = content_filter.get("violence") {
            results.violence = Some(ContentFilterResult {
                filtered: violence
                    .get("filtered")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                severity: violence
                    .get("severity")
                    .and_then(|v| v.as_str())
                    .and_then(parse_severity),
            });
        }

        AzureOpenAIError::ContentFiltered {
            filter_results: results,
            message: body.to_string(),
        }
    }
}

fn parse_severity(s: &str) -> Option<ContentFilterSeverity> {
    match s {
        "safe" => Some(ContentFilterSeverity::Safe),
        "low" => Some(ContentFilterSeverity::Low),
        "medium" => Some(ContentFilterSeverity::Medium),
        "high" => Some(ContentFilterSeverity::High),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = AzureOpenAIError::Authentication {
            message: "Invalid API key".to_string(),
        };
        assert!(err.to_string().contains("Authentication failed"));
    }

    #[test]
    fn test_is_retryable() {
        let rate_limit = AzureOpenAIError::RateLimit {
            retry_after: None,
            message: "Too many requests".to_string(),
        };
        assert!(rate_limit.is_retryable());

        let auth = AzureOpenAIError::Authentication {
            message: "Invalid key".to_string(),
        };
        assert!(!auth.is_retryable());
    }

    #[test]
    fn test_content_filter_error() {
        let err = AzureOpenAIError::ContentFiltered {
            filter_results: ContentFilterResults::default(),
            message: "Content filtered".to_string(),
        };
        assert!(err.is_content_filtered());
    }
}
