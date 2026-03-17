//! Gemini SDK errors

use thiserror::Error;

/// Gemini SDK error types
#[derive(Error, Debug)]
pub enum GeminiError {
    #[error("API error: HTTP {status} - {message}")]
    ApiError { status: reqwest::StatusCode, message: String },
    
    #[error("Request failed: {0}")]
    RequestError(#[from] reqwest::Error),
    
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
    
    #[error("Invalid API key")]
    InvalidApiKey,
    
    #[error("Model not found: {0}")]
    ModelNotFound(String),
    
    #[error("Content blocked: {0}")]
    ContentBlocked(String),
    
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    
    #[error("Timeout")]
    Timeout,
    
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    
    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl GeminiError {
    /// Check if the error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            GeminiError::RateLimitExceeded | GeminiError::Timeout => true,
            GeminiError::ApiError { status, .. } => status.as_u16() >= 500,
            _ => false,
        }
    }
    
    /// Get the HTTP status code if available
    pub fn status_code(&self) -> Option<reqwest::StatusCode> {
        match self {
            GeminiError::ApiError { status, .. } => Some(*status),
            _ => None,
        }
    }
}
