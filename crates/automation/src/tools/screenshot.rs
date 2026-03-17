//! Screenshot tool for browser automation.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::info;

use crate::browser::{Browser, ScreenshotOptions};
use crate::tools::{AutomationTool, ToolContext, error_response, success_response};

/// Take a screenshot of the page.
pub struct ScreenshotTool {
    browser: Browser,
}

impl ScreenshotTool {
    /// Create a new screenshot tool.
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
impl AutomationTool for ScreenshotTool {
    fn name(&self) -> &str {
        "browser_screenshot"
    }

    fn description(&self) -> &str {
        "Take a screenshot of the current page or a specific element. \
         Screenshots can be saved to a file or returned as base64 data. \
         Supports full page screenshots and element-specific screenshots."
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "selector": {
                    "type": "string",
                    "description": "CSS selector for a specific element to screenshot (if not provided, screenshots the full page)"
                },
                "path": {
                    "type": "string",
                    "description": "File path to save the screenshot (optional, will return base64 if not provided)"
                },
                "full_page": {
                    "type": "boolean",
                    "description": "Whether to capture the full page or just the viewport",
                    "default": false
                },
                "format": {
                    "type": "string",
                    "enum": ["png", "jpeg"],
                    "description": "Image format",
                    "default": "png"
                },
                "quality": {
                    "type": "integer",
                    "description": "JPEG quality (0-100, only for jpeg format)",
                    "default": 80
                },
                "omit_background": {
                    "type": "boolean",
                    "description": "Whether to hide the default white background for transparent screenshots",
                    "default": false
                }
            }
        })
    }

    async fn execute(&self, input: serde_json::Value, ctx: &ToolContext) -> anyhow::Result<String> {
        let args: ScreenshotArgs = serde_json::from_value(input)?;

        let pages = self
            .browser
            .pages()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get pages: {}", e))?;

        let page = pages
            .first()
            .ok_or_else(|| anyhow::anyhow!("No pages available"))?;

        info!("Taking screenshot");

        // Build screenshot options
        let format = match args.format.as_deref() {
            Some("jpeg") | Some("jpg") => crate::browser::ScreenshotFormat::Jpeg,
            _ => crate::browser::ScreenshotFormat::Png,
        };

        let options = ScreenshotOptions {
            format,
            quality: args.quality.map(|q| q.min(100)),
            clip: None,
            full_page: args.full_page.unwrap_or(false),
            hide_selectors: vec![],
        };

        // Take screenshot
        let screenshot = if let Some(selector) = args.selector {
            // Element-specific screenshot
            let element = page
                .query_selector(&selector)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to find element: {}", e))?;

            match element {
                Some(el) => el
                    .screenshot()
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to take element screenshot: {}", e))?,
                None => {
                    return Ok(error_response(format!("Element not found: {}", selector)));
                }
            }
        } else {
            // Full page screenshot
            page.screenshot_with_options(options)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to take screenshot: {}", e))?
        };

        // Save or return base64
        let result = if let Some(path) = args.path {
            // Save to file
            let path = if path.starts_with('/') {
                path
            } else if let Some(workspace) = &ctx.workspace_path {
                format!("{}/{}", workspace, path)
            } else {
                path
            };

            screenshot
                .save(&path)
                .map_err(|e| anyhow::anyhow!("Failed to save screenshot: {}", e))?;

            success_response(format!(
                "Screenshot saved to: {}\nDimensions: {}x{}",
                path, screenshot.width, screenshot.height
            ))
        } else {
            // Return base64
            #[cfg(feature = "base64")]
            {
                let base64_data = screenshot.to_base64();
                success_response(format!(
                    "Screenshot captured ({}x{} pixels)\nFormat: {:?}\nBase64 length: {} characters",
                    screenshot.width,
                    screenshot.height,
                    format,
                    base64_data.len()
                ))
            }
            #[cfg(not(feature = "base64"))]
            {
                success_response(format!(
                    "Screenshot captured ({}x{} pixels)\nFormat: {:?}\nNote: Enable 'base64' feature for base64 output",
                    screenshot.width, screenshot.height, format
                ))
            }
        };

        Ok(result)
    }
}

/// Arguments for screenshot tool.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScreenshotArgs {
    /// CSS selector for element.
    #[serde(default)]
    pub selector: Option<String>,
    /// File path to save.
    #[serde(default)]
    pub path: Option<String>,
    /// Whether to capture full page.
    #[serde(default)]
    pub full_page: Option<bool>,
    /// Image format.
    #[serde(default)]
    pub format: Option<String>,
    /// JPEG quality.
    #[serde(default)]
    pub quality: Option<u8>,
    /// Omit background.
    #[serde(default)]
    pub omit_background: Option<bool>,
}
