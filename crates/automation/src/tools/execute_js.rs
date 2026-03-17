//! Execute JavaScript tool for browser automation.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{debug, info, warn};

use crate::browser::Browser;
use crate::tools::{error_response, success_response, AutomationTool, ToolContext};

/// Execute JavaScript in the browser.
pub struct ExecuteJsTool {
    browser: Browser,
}

impl ExecuteJsTool {
    /// Create a new execute JS tool.
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
impl AutomationTool for ExecuteJsTool {
    fn name(&self) -> &str {
        "browser_execute_js"
    }

    fn description(&self) -> &str {
        "Execute JavaScript code in the browser context. \
         This provides full access to the page's DOM and JavaScript environment. \
         The script runs in the context of the current page. \
         Returns the result of the last expression in the script."
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "script": {
                    "type": "string",
                    "description": "JavaScript code to execute. Can be a single expression or multiple statements. \
                                   The last expression's value will be returned. \
                                   Use 'return' explicitly if needed."
                },
                "selector": {
                    "type": "string",
                    "description": "Optional CSS selector. If provided, the script will run with the element as 'this'"
                },
                "args": {
                    "type": "array",
                    "description": "Arguments to pass to the script (accessible as arguments[0], arguments[1], etc.)",
                    "items": {
                        "type": ["string", "number", "boolean", "object", "array", "null"]
                    }
                },
                "timeout": {
                    "type": "integer",
                    "description": "Maximum time to wait for script execution in milliseconds",
                    "default": 30000
                }
            },
            "required": ["script"]
        })
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _ctx: &ToolContext,
    ) -> anyhow::Result<String> {
        let args: ExecuteJsArgs = serde_json::from_value(input)?;
        
        let pages = self.browser.pages().await.map_err(|e| {
            anyhow::anyhow!("Failed to get pages: {}", e)
        })?;
        
        let page = pages.first().ok_or_else(|| {
            anyhow::anyhow!("No pages available")
        })?;
        
        info!(script_len = args.script.len(), "Executing JavaScript");
        
        let result = if let Some(selector) = args.selector {
            // Execute in element context
            let element = page.query_selector(&selector).await.map_err(|e| {
                anyhow::anyhow!("Failed to find element: {}", e)
            })?;
            
            match element {
                Some(el) => {
                    let script = format!(
                        "(function() {{ {} }}).call(this)",
                        args.script
                    );
                    el.evaluate(&script).await.map_err(|e| {
                        anyhow::anyhow!("Failed to execute script on element: {}", e)
                    })?
                }
                None => {
                    return Ok(error_response(format!("Element not found: {}", selector)));
                }
            }
        } else {
            // Execute in page context
            if let Some(args_vec) = args.args {
                page.evaluate_with_args(&args.script, args_vec).await.map_err(|e| {
                    anyhow::anyhow!("Failed to execute script with args: {}", e)
                })?
            } else {
                page.evaluate(&args.script).await.map_err(|e| {
                    anyhow::anyhow!("Failed to execute script: {}", e)
                })?
            }
        };
        
        // Format the result
        let result_str = match &result {
            serde_json::Value::Null => "null".to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::String(s) => format!("\"{}\"", s),
            serde_json::Value::Array(arr) => {
                serde_json::to_string_pretty(arr).unwrap_or_else(|_| "[Array]".to_string())
            }
            serde_json::Value::Object(obj) => {
                serde_json::to_string_pretty(obj).unwrap_or_else(|_| "{Object}".to_string())
            }
        };
        
        Ok(success_response(format!(
            "Script executed successfully.\n\nResult ({}):\n{}",
            get_json_type(&result),
            result_str
        )))
    }
}

/// Get a human-readable type name for a JSON value.
fn get_json_type(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(n) if n.is_i64() || n.is_u64() => "integer",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

/// Arguments for execute JS tool.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExecuteJsArgs {
    /// JavaScript code to execute.
    pub script: String,
    /// Optional element selector.
    #[serde(default)]
    pub selector: Option<String>,
    /// Arguments to pass.
    #[serde(default)]
    pub args: Option<Vec<serde_json::Value>>,
    /// Timeout in milliseconds.
    #[serde(default)]
    pub timeout: Option<u64>,
}
