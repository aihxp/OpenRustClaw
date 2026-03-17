//! Error types for the Canvas crate

use thiserror::Error;

/// Result type alias for canvas operations
pub type CanvasResult<T> = Result<T, CanvasError>;

/// Errors that can occur in canvas operations
#[derive(Error, Debug)]
pub enum CanvasError {
    /// Canvas not found
    #[error("Canvas not found: {0}")]
    CanvasNotFound(String),

    /// Element not found in canvas
    #[error("Element not found: {0}")]
    ElementNotFound(String),

    /// Invalid message format
    #[error("Invalid message format: {0}")]
    InvalidMessage(String),

    /// WebSocket error
    #[error("WebSocket error: {0}")]
    WebSocket(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Channel send error
    #[error("Channel send error")]
    ChannelSend,

    /// Invalid canvas state
    #[error("Invalid canvas state: {0}")]
    InvalidState(String),
}

impl CanvasError {
    /// Create a canvas not found error
    pub fn canvas_not_found(id: impl Into<String>) -> Self {
        CanvasError::CanvasNotFound(id.into())
    }

    /// Create an element not found error
    pub fn element_not_found(id: impl Into<String>) -> Self {
        CanvasError::ElementNotFound(id.into())
    }

    /// Create an invalid message error
    pub fn invalid_message(msg: impl Into<String>) -> Self {
        CanvasError::InvalidMessage(msg.into())
    }
}

// Axum response support
impl axum::response::IntoResponse for CanvasError {
    fn into_response(self) -> axum::response::Response {
        use axum::http::StatusCode;

        let status = match &self {
            CanvasError::CanvasNotFound(_) => StatusCode::NOT_FOUND,
            CanvasError::ElementNotFound(_) => StatusCode::NOT_FOUND,
            CanvasError::InvalidMessage(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (status, self.to_string()).into_response()
    }
}
