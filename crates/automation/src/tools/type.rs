//! Type tool for browser automation.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{debug, info};

use crate::browser::Browser;
use crate::tools::{AutomationTool, ToolContext, error_response, success_response};

/// Type text into an element.
pub struct TypeTool {
    browser: Browser,
}

impl TypeTool {
    /// Create a new type tool.
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
impl AutomationTool for TypeTool {
    fn name(&self) -> &str {
        "browser_type"
    }

    fn description(&self) -> &str {
        "Type text into an input element in the browser. \
         This can be used to fill forms, search boxes, and other text inputs. \
         The element is identified by a CSS selector. \
         By default, this will clear any existing text before typing."
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "selector": {
                    "type": "string",
                    "description": "CSS selector for the input element (e.g., '#email', 'input[name=\"username\"]')"
                },
                "text": {
                    "type": "string",
                    "description": "The text to type into the element"
                },
                "clear": {
                    "type": "boolean",
                    "description": "Whether to clear the existing content before typing",
                    "default": true
                },
                "delay": {
                    "type": "integer",
                    "description": "Delay between keystrokes in milliseconds (0 for instant)",
                    "default": 0
                },
                "submit": {
                    "type": "boolean",
                    "description": "Whether to submit the form after typing (by pressing Enter)",
                    "default": false
                },
                "wait_for": {
                    "type": "boolean",
                    "description": "Whether to wait for the element to be visible",
                    "default": true
                }
            },
            "required": ["selector", "text"]
        })
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _ctx: &ToolContext,
    ) -> anyhow::Result<String> {
        let args: TypeArgs = serde_json::from_value(input)?;

        let pages = self
            .browser
            .pages()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get pages: {}", e))?;

        let page = pages
            .first()
            .ok_or_else(|| anyhow::anyhow!("No pages available"))?;

        info!(selector = %args.selector, text_len = args.text.len(), "Typing text");

        // Wait for element if requested
        if args.wait_for {
            match page.wait_for_selector(&args.selector).await {
                Ok(_) => {}
                Err(e) => {
                    return Ok(error_response(format!(
                        "Element not found: {} - {}",
                        args.selector, e
                    )));
                }
            }
        }

        // Get element
        let element = match page.query_selector(&args.selector).await {
            Ok(Some(el)) => el,
            Ok(None) => {
                return Ok(error_response(format!(
                    "Element not found: {}",
                    args.selector
                )));
            }
            Err(e) => {
                return Ok(error_response(format!("Failed to find element: {}", e)));
            }
        };

        // Check if it's an input element
        let tag_name = element.tag_name().await.unwrap_or_default();
        let input_type = element.get_attribute("type").await.ok().flatten();

        // Clear existing content if requested
        if args.clear {
            element
                .clear()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to clear element: {}", e))?;
            debug!("Cleared existing content");
        }

        // Type the text
        if let Some(delay) = args.delay {
            if delay > 0 {
                // Type with delay (character by character)
                for ch in args.text.chars() {
                    element
                        .type_text(&ch.to_string())
                        .await
                        .map_err(|e| anyhow::anyhow!("Failed to type character: {}", e))?;
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                }
            } else {
                element
                    .type_text(&args.text)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to type text: {}", e))?;
            }
        } else {
            element
                .type_text(&args.text)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to type text: {}", e))?;
        }

        // Submit form if requested
        if args.submit {
            element
                .press("Enter")
                .await
                .map_err(|e| anyhow::anyhow!("Failed to submit form: {}", e))?;
            info!("Form submitted");
        }

        Ok(success_response(format!(
            "Successfully typed '{}' into {} element (type: {:?})",
            args.text, tag_name, input_type
        )))
    }
}

/// Arguments for type tool.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TypeArgs {
    /// CSS selector.
    pub selector: String,
    /// Text to type.
    pub text: String,
    /// Whether to clear existing content.
    #[serde(default = "default_true")]
    pub clear: bool,
    /// Delay between keystrokes.
    #[serde(default)]
    pub delay: Option<u64>,
    /// Whether to submit after typing.
    #[serde(default)]
    pub submit: bool,
    /// Whether to wait for element.
    #[serde(default = "default_true")]
    pub wait_for: bool,
}

fn default_true() -> bool {
    true
}
