//! Extract content tool for browser automation.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{debug, info};

use crate::browser::Browser;
use crate::tools::{error_response, success_response, AutomationTool, ToolContext};

/// Extract content from the page.
pub struct ExtractContentTool {
    browser: Browser,
}

impl ExtractContentTool {
    /// Create a new extract content tool.
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
impl AutomationTool for ExtractContentTool {
    fn name(&self) -> &str {
        "browser_extract"
    }

    fn description(&self) -> &str {
        "Extract content from the current page. \
         Can extract text, HTML, links, images, or structured data. \
         Useful for web scraping and data extraction tasks."
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "what": {
                    "type": "string",
                    "enum": ["text", "html", "links", "images", "headings", "tables", "forms", "selector"],
                    "description": "What content to extract",
                    "default": "text"
                },
                "selector": {
                    "type": "string",
                    "description": "CSS selector to scope the extraction (when 'what' is 'selector')"
                },
                "url": {
                    "type": "boolean",
                    "description": "Include URL with each extracted item",
                    "default": false
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum number of results to return",
                    "default": 100
                }
            }
        })
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _ctx: &ToolContext,
    ) -> anyhow::Result<String> {
        let args: ExtractArgs = serde_json::from_value(input)?;
        
        let pages = self.browser.pages().await.map_err(|e| {
            anyhow::anyhow!("Failed to get pages: {}", e)
        })?;
        
        let page = pages.first().ok_or_else(|| {
            anyhow::anyhow!("No pages available")
        })?;
        
        info!(what = ?args.what, "Extracting content");
        
        let result = match args.what.as_deref() {
            Some("text") | None => {
                // Extract all visible text
                let script = r#"
                    (function() {
                        const walker = document.createTreeWalker(
                            document.body,
                            NodeFilter.SHOW_TEXT,
                            null,
                            false
                        );
                        let text = '';
                        let node;
                        while (node = walker.nextNode()) {
                            const parent = node.parentElement;
                            if (parent && getComputedStyle(parent).display !== 'none') {
                                text += node.textContent + ' ';
                            }
                        }
                        return text.trim().replace(/\s+/g, ' ');
                    })()
                "#;
                let text = page.evaluate(script).await.map_err(|e| {
                    anyhow::anyhow!("Failed to extract text: {}", e)
                })?;
                
                let text_str = text.as_str().unwrap_or("");
                let preview: String = text_str.chars().take(2000).collect();
                
                if text_str.len() > 2000 {
                    format!("Extracted text (first 2000 chars):\n{}...\n\n[{} characters total]", 
                        preview, text_str.len())
                } else {
                    format!("Extracted text:\n{}", text_str)
                }
            }
            
            Some("html") => {
                let html = page.content().await.map_err(|e| {
                    anyhow::anyhow!("Failed to get HTML: {}", e)
                })?;
                
                let preview: String = html.chars().take(2000).collect();
                
                if html.len() > 2000 {
                    format!("HTML content (first 2000 chars):\n{}...\n\n[{} characters total]", 
                        preview, html.len())
                } else {
                    format!("HTML content:\n{}", html)
                }
            }
            
            Some("links") => {
                let script = r#"
                    Array.from(document.querySelectorAll('a[href]')).map(a => ({
                        text: a.textContent.trim(),
                        href: a.href,
                        title: a.title || ''
                    }))
                "#;
                let links = page.evaluate(script).await.map_err(|e| {
                    anyhow::anyhow!("Failed to extract links: {}", e)
                })?;
                
                let max = args.max_results.unwrap_or(100);
                format!("Extracted links:\n{}", 
                    serde_json::to_string_pretty(&links).unwrap_or_default())
            }
            
            Some("images") => {
                let script = r#"
                    Array.from(document.querySelectorAll('img')).map(img => ({
                        src: img.src,
                        alt: img.alt || '',
                        width: img.naturalWidth,
                        height: img.naturalHeight
                    })).filter(img => img.src)
                "#;
                let images = page.evaluate(script).await.map_err(|e| {
                    anyhow::anyhow!("Failed to extract images: {}", e)
                })?;
                
                format!("Extracted images:\n{}", 
                    serde_json::to_string_pretty(&images).unwrap_or_default())
            }
            
            Some("headings") => {
                let script = r#"
                    Array.from(document.querySelectorAll('h1, h2, h3, h4, h5, h6')).map(h => ({
                        level: parseInt(h.tagName[1]),
                        text: h.textContent.trim()
                    }))
                "#;
                let headings = page.evaluate(script).await.map_err(|e| {
                    anyhow::anyhow!("Failed to extract headings: {}", e)
                })?;
                
                format!("Extracted headings:\n{}", 
                    serde_json::to_string_pretty(&headings).unwrap_or_default())
            }
            
            Some("tables") => {
                let script = r#"
                    Array.from(document.querySelectorAll('table')).map((table, i) => ({
                        index: i,
                        caption: table.caption?.textContent?.trim() || '',
                        rows: Array.from(table.rows).slice(0, 10).map(row => 
                            Array.from(row.cells).slice(0, 10).map(cell => cell.textContent.trim())
                        )
                    }))
                "#;
                let tables = page.evaluate(script).await.map_err(|e| {
                    anyhow::anyhow!("Failed to extract tables: {}", e)
                })?;
                
                format!("Extracted tables:\n{}", 
                    serde_json::to_string_pretty(&tables).unwrap_or_default())
            }
            
            Some("forms") => {
                let script = r#"
                    Array.from(document.querySelectorAll('form')).map((form, i) => ({
                        index: i,
                        action: form.action || '',
                        method: form.method || 'GET',
                        inputs: Array.from(form.querySelectorAll('input, select, textarea')).map(input => ({
                            name: input.name || '',
                            type: input.type || input.tagName.toLowerCase(),
                            required: input.required,
                            placeholder: input.placeholder || ''
                        }))
                    }))
                "#;
                let forms = page.evaluate(script).await.map_err(|e| {
                    anyhow::anyhow!("Failed to extract forms: {}", e)
                })?;
                
                format!("Extracted forms:\n{}", 
                    serde_json::to_string_pretty(&forms).unwrap_or_default())
            }
            
            Some("selector") => {
                let selector = args.selector.ok_or_else(|| {
                    anyhow::anyhow!("'selector' parameter required when 'what' is 'selector'")
                })?;
                
                let script = format!(
                    r#"Array.from(document.querySelectorAll('{}')).map(el => ({{
                        tag: el.tagName.toLowerCase(),
                        text: el.textContent.trim().substring(0, 500),
                        html: el.innerHTML.substring(0, 1000)
                    }}))"#,
                    selector.replace('\\', "\\\\").replace('\'', "\\'")
                );
                
                let elements = page.evaluate(&script).await.map_err(|e| {
                    anyhow::anyhow!("Failed to extract elements: {}", e)
                })?;
                
                format!("Extracted elements matching '{}':\n{}", 
                    selector,
                    serde_json::to_string_pretty(&elements).unwrap_or_default())
            }
            
            _ => {
                return Ok(error_response(format!("Unknown extraction type: {:?}", args.what)));
            }
        };
        
        Ok(success_response(result))
    }
}

/// Arguments for extract content tool.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExtractArgs {
    /// What to extract.
    #[serde(default)]
    pub what: Option<String>,
    /// CSS selector for custom extraction.
    #[serde(default)]
    pub selector: Option<String>,
    /// Include URLs.
    #[serde(default)]
    pub url: Option<bool>,
    /// Maximum results.
    #[serde(default = "default_max_results")]
    pub max_results: Option<usize>,
}

fn default_max_results() -> Option<usize> {
    Some(100)
}
