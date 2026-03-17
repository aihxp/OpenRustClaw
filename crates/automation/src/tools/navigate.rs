//! Navigate tool for browser automation.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::info;

use crate::browser::Browser;
use crate::tools::{success_response, AutomationTool, ToolContext};

/// Navigate to a URL.
pub struct NavigateTool {
    browser: Browser,
}

impl NavigateTool {
    /// Create a new navigate tool.
    pub fn new(browser: &Browser) -> Self {
        Self {
            browser: Browser {
                inner: browser.inner.clone(),
                config: browser.config().clone(),
            },
        }
    }
}

#[async_trait]
impl AutomationTool for NavigateTool {
    fn name(&self) -> &str {
        "browser_navigate"
    }

    fn description(&self) -> &str {
        "Navigate to a URL in the browser. \
         This will load the specified web page and wait for it to finish loading. \
         The URL must be a valid HTTP or HTTPS URL."
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The URL to navigate to (must be a valid HTTP or HTTPS URL)"
                },
                "wait_until": {
                    "type": "string",
                    "enum": ["load", "domcontentloaded", "networkidle"],
                    "description": "When to consider navigation complete",
                    "default": "load"
                },
                "timeout": {
                    "type": "integer",
                    "description": "Navigation timeout in milliseconds",
                    "default": 30000
                }
            },
            "required": ["url"]
        })
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _ctx: &ToolContext,
    ) -> anyhow::Result<String> {
        let args: NavigateArgs = serde_json::from_value(input)?;
        
        info!(url = %args.url, "Navigating to URL");
        
        // Validate URL
        let url = if args.url.starts_with("http://") || args.url.starts_with("https://") {
            args.url
        } else {
            format!("https://{}", args.url)
        };
        
        // Navigate
        let page = self.browser.new_page().await.map_err(|e| {
            anyhow::anyhow!("Failed to create page: {}", e)
        })?;
        
        page.goto(&url).await.map_err(|e| {
            anyhow::anyhow!("Failed to navigate: {}", e)
        })?;
        
        // Wait for specified load state
        let load_state = match args.wait_until.as_deref() {
            Some("domcontentloaded") => crate::browser::LoadState::DomContentLoaded,
            Some("networkidle") => crate::browser::LoadState::NetworkIdle,
            _ => crate::browser::LoadState::Load,
        };
        
        page.wait_for_load_state(load_state).await.map_err(|e| {
            anyhow::anyhow!("Failed to wait for load state: {}", e)
        })?;
        
        // Get page info
        let title = page.title().await.unwrap_or_default();
        let final_url = page.url().await.unwrap_or_default();
        
        Ok(success_response(format!(
            "Successfully navigated to: {}\nTitle: {}",
            final_url, title
        )))
    }
}

/// Arguments for navigate tool.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NavigateArgs {
    /// URL to navigate to.
    pub url: String,
    /// When to consider navigation complete.
    #[serde(default)]
    pub wait_until: Option<String>,
    /// Navigation timeout in milliseconds.
    #[serde(default)]
    pub timeout: Option<u64>,
}
