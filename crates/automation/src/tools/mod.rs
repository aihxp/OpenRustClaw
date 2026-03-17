//! Automation tools for agent use.
//!
//! This module provides tools that can be registered with the agent's
//! skill registry to enable browser automation capabilities.
//!
//! # Example
//!
//! ```rust,no_run
//! use openrustclaw_automation::tools::*;
//! use openrustclaw_automation::Browser;
//!
//! async fn setup_tools(browser: &Browser) -> Vec<Box<dyn AutomationTool>> {
//!     vec![
//!         Box::new(NavigateTool::new(browser)),
//!         Box::new(ClickTool::new(browser)),
//!         Box::new(TypeTool::new(browser)),
//!         Box::new(ScreenshotTool::new(browser)),
//!     ]
//! }
//! ```

pub mod click;
pub mod download;
pub mod execute_js;
pub mod extract;
pub mod navigate;
pub mod pdf;
pub mod screenshot;
pub mod r#type;

// Re-exports
pub use click::ClickTool;
pub use download::DownloadTool;
pub use execute_js::ExecuteJsTool;
pub use extract::ExtractContentTool;
pub use navigate::NavigateTool;
pub use pdf::PdfTool;
pub use screenshot::ScreenshotTool;
pub use r#type::TypeTool;

use async_trait::async_trait;
use serde_json::Value;

/// Context for tool execution.
#[derive(Debug, Clone)]
pub struct ToolContext {
    /// Current session ID.
    pub session_id: String,
    /// Current user ID.
    pub user_id: String,
    /// Workspace path (if applicable).
    pub workspace_path: Option<String>,
}

/// Trait for automation tools.
///
/// This is a simplified version that matches the core Tool trait.
/// Tools can be registered with the agent for browser automation.
#[async_trait]
pub trait AutomationTool: Send + Sync {
    /// Tool name.
    fn name(&self) -> &str;

    /// Tool description for the LLM.
    fn description(&self) -> &str;

    /// JSON Schema for the tool's input parameters.
    fn schema(&self) -> Value;

    /// Execute the tool.
    async fn execute(&self, input: Value, ctx: &ToolContext) -> anyhow::Result<String>;
}

/// Parse and validate input against a schema.
pub fn parse_input<T: serde::de::DeserializeOwned>(input: Value) -> anyhow::Result<T> {
    serde_json::from_value(input).map_err(|e| anyhow::anyhow!("Invalid input: {}", e))
}

/// Helper to create a success response.
pub fn success_response(message: impl Into<String>) -> String {
    message.into()
}

/// Helper to create an error response.
pub fn error_response(error: impl Into<String>) -> String {
    format!("Error: {}", error.into())
}
