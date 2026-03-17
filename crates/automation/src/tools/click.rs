//! Click tool for browser automation.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{info, warn};

use crate::browser::Browser;
use crate::tools::{AutomationTool, ToolContext, error_response, success_response};

/// Click on an element.
pub struct ClickTool {
    browser: Browser,
}

impl ClickTool {
    /// Create a new click tool.
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
impl AutomationTool for ClickTool {
    fn name(&self) -> &str {
        "browser_click"
    }

    fn description(&self) -> &str {
        "Click on an element in the browser. \
         The element can be identified by a CSS selector, XPath, or text content. \
         The click will be performed in the center of the element. \
         Returns information about the element that was clicked."
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "selector": {
                    "type": "string",
                    "description": "CSS selector to identify the element (e.g., '#submit-button', '.btn-primary', 'button[type=\"submit\"]')"
                },
                "xpath": {
                    "type": "string",
                    "description": "XPath expression to identify the element (alternative to selector)"
                },
                "text": {
                    "type": "string",
                    "description": "Text content to find and click (will click first matching element)"
                },
                "button": {
                    "type": "string",
                    "enum": ["left", "right", "middle"],
                    "description": "Mouse button to click",
                    "default": "left"
                },
                "click_count": {
                    "type": "integer",
                    "description": "Number of clicks (1 for single click, 2 for double click)",
                    "default": 1
                },
                "wait_for": {
                    "type": "boolean",
                    "description": "Whether to wait for the element to be visible before clicking",
                    "default": true
                },
                "timeout": {
                    "type": "integer",
                    "description": "Timeout in milliseconds to wait for the element",
                    "default": 5000
                }
            },
            "oneOf": [
                { "required": ["selector"] },
                { "required": ["xpath"] },
                { "required": ["text"] }
            ]
        })
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _ctx: &ToolContext,
    ) -> anyhow::Result<String> {
        let args: ClickArgs = serde_json::from_value(input)?;

        // Get current page (in a real implementation, we'd track the active page)
        let pages = self
            .browser
            .pages()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get pages: {}", e))?;

        let page = pages
            .first()
            .ok_or_else(|| anyhow::anyhow!("No pages available"))?;

        // Determine selector
        let selector = if let Some(sel) = args.selector {
            sel
        } else if let Some(xpath) = args.xpath {
            // Convert XPath to a selector (in real impl, use proper XPath support)
            warn!("XPath support is limited, using CSS selector fallback");
            xpath
        } else if let Some(text) = args.text {
            // Build a selector that finds elements by text content
            format!("text={}", text)
        } else {
            return Ok(error_response("No selector, xpath, or text provided"));
        };

        info!(selector = %selector, "Clicking element");

        // Wait for element if requested
        if args.wait_for {
            let timeout = args.timeout.unwrap_or(5000);
            match page.wait_for_selector(&selector).await {
                Ok(_) => {}
                Err(e) => {
                    return Ok(error_response(format!(
                        "Element not found within {}ms: {}",
                        timeout, e
                    )));
                }
            }
        }

        // Get element info before clicking
        let element = match page.query_selector(&selector).await {
            Ok(Some(el)) => el,
            Ok(None) => {
                return Ok(error_response(format!("Element not found: {}", selector)));
            }
            Err(e) => {
                return Ok(error_response(format!("Failed to find element: {}", e)));
            }
        };

        let tag_name = element.tag_name().await.unwrap_or_default();
        let text = element.inner_text().await.unwrap_or_default();

        // Perform the click
        let click_count = args.click_count.unwrap_or(1);
        if click_count == 2 {
            element
                .dblclick()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to double-click element: {}", e))?;
        } else {
            element
                .click()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to click element: {}", e))?;
        }

        Ok(success_response(format!(
            "Successfully clicked element:\n  Selector: {}\n  Tag: {}\n  Text: {}",
            selector, tag_name, text
        )))
    }
}

/// Arguments for click tool.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ClickArgs {
    /// CSS selector.
    #[serde(default)]
    pub selector: Option<String>,
    /// XPath expression.
    #[serde(default)]
    pub xpath: Option<String>,
    /// Text content to find.
    #[serde(default)]
    pub text: Option<String>,
    /// Mouse button.
    #[serde(default)]
    pub button: Option<String>,
    /// Click count.
    #[serde(default)]
    pub click_count: Option<u32>,
    /// Whether to wait for element.
    #[serde(default = "default_true")]
    pub wait_for: bool,
    /// Timeout in milliseconds.
    #[serde(default)]
    pub timeout: Option<u64>,
}

fn default_true() -> bool {
    true
}
