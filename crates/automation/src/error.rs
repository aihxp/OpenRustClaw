//! Error types for browser automation.

use std::fmt;

/// Result type alias for automation operations.
pub type Result<T> = std::result::Result<T, AutomationError>;

/// Errors that can occur during browser automation.
#[derive(Debug)]
pub enum AutomationError {
    /// Browser failed to launch.
    LaunchFailed {
        browser: String,
        reason: String,
    },

    /// Navigation failed.
    NavigationFailed {
        url: String,
        reason: String,
    },

    /// Element not found.
    ElementNotFound {
        selector: String,
    },

    /// Element interaction failed.
    InteractionFailed {
        action: String,
        selector: String,
        reason: String,
    },

    /// Screenshot failed.
    ScreenshotFailed {
        reason: String,
    },

    /// JavaScript execution failed.
    JavaScriptError {
        code: String,
        error: String,
    },

    /// Network error.
    NetworkError {
        url: String,
        status: Option<u16>,
        message: String,
    },

    /// Timeout error.
    Timeout {
        operation: String,
        duration_ms: u64,
    },

    /// CDP-specific error.
    CdpError {
        method: String,
        error: String,
    },

    /// Playwright-specific error.
    #[cfg(feature = "playwright")]
    PlaywrightError {
        message: String,
    },

    /// Configuration error.
    ConfigError {
        field: String,
        reason: String,
    },

    /// PDF generation error.
    PdfError {
        reason: String,
    },

    /// Generic IO error.
    Io(std::io::Error),

    /// Serialization error.
    Serialization(serde_json::Error),

    /// Other errors.
    Other(String),
}

impl fmt::Display for AutomationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LaunchFailed { browser, reason } => {
                write!(f, "Failed to launch {}: {}", browser, reason)
            }
            Self::NavigationFailed { url, reason } => {
                write!(f, "Failed to navigate to {}: {}", url, reason)
            }
            Self::ElementNotFound { selector } => {
                write!(f, "Element not found: {}", selector)
            }
            Self::InteractionFailed {
                action,
                selector,
                reason,
            } => {
                write!(f, "Failed to {} element ({}): {}", action, selector, reason)
            }
            Self::ScreenshotFailed { reason } => {
                write!(f, "Screenshot failed: {}", reason)
            }
            Self::JavaScriptError { code, error } => {
                write!(f, "JavaScript execution failed: {}\nCode: {}", error, code)
            }
            Self::NetworkError {
                url,
                status,
                message,
            } => {
                if let Some(status) = status {
                    write!(f, "Network error ({}): {} - {}", status, url, message)
                } else {
                    write!(f, "Network error: {} - {}", url, message)
                }
            }
            Self::Timeout {
                operation,
                duration_ms,
            } => {
                write!(f, "Timeout after {}ms: {}", duration_ms, operation)
            }
            Self::CdpError { method, error } => {
                write!(f, "CDP error in {}: {}", method, error)
            }
            #[cfg(feature = "playwright")]
            Self::PlaywrightError { message } => {
                write!(f, "Playwright error: {}", message)
            }
            Self::ConfigError { field, reason } => {
                write!(f, "Configuration error for '{}': {}", field, reason)
            }
            Self::PdfError { reason } => {
                write!(f, "PDF generation error: {}", reason)
            }
            Self::Io(err) => write!(f, "IO error: {}", err),
            Self::Serialization(err) => write!(f, "Serialization error: {}", err),
            Self::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for AutomationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Serialization(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for AutomationError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<serde_json::Error> for AutomationError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serialization(err)
    }
}

impl From<String> for AutomationError {
    fn from(msg: String) -> Self {
        Self::Other(msg)
    }
}

impl From<&str> for AutomationError {
    fn from(msg: &str) -> Self {
        Self::Other(msg.to_string())
    }
}
