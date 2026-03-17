//! PDF generation tool for browser automation.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::info;

use crate::browser::{Browser, PdfOptions};
use crate::tools::{success_response, AutomationTool, ToolContext};

/// Generate a PDF from the current page.
pub struct PdfTool {
    browser: Browser,
}

impl PdfTool {
    /// Create a new PDF tool.
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
impl AutomationTool for PdfTool {
    fn name(&self) -> &str {
        "browser_pdf"
    }

    fn description(&self) -> &str {
        "Generate a PDF from the current page. \
         Supports various paper sizes (A4, Letter, etc.) and custom dimensions. \
         Can include headers, footers, and background graphics."
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "File path to save the PDF (required)"
                },
                "format": {
                    "type": "string",
                    "enum": ["A0", "A1", "A2", "A3", "A4", "A5", "A6", "Legal", "Letter", "Tabloid", "Ledger"],
                    "description": "Paper format",
                    "default": "A4"
                },
                "width": {
                    "type": "string",
                    "description": "Custom paper width (e.g., '8.5in', '210mm') - overrides format"
                },
                "height": {
                    "type": "string",
                    "description": "Custom paper height (e.g., '11in', '297mm') - overrides format"
                },
                "print_background": {
                    "type": "boolean",
                    "description": "Whether to print background graphics",
                    "default": true
                },
                "landscape": {
                    "type": "boolean",
                    "description": "Whether to use landscape orientation",
                    "default": false
                },
                "scale": {
                    "type": "number",
                    "description": "Scale factor (0.1 to 2.0)",
                    "default": 1.0
                },
                "margin": {
                    "type": "object",
                    "properties": {
                        "top": {
                            "type": "string",
                            "description": "Top margin (e.g., '1in', '20mm')",
                            "default": "0.4in"
                        },
                        "bottom": {
                            "type": "string",
                            "description": "Bottom margin (e.g., '1in', '20mm')",
                            "default": "0.4in"
                        },
                        "left": {
                            "type": "string",
                            "description": "Left margin (e.g., '1in', '20mm')",
                            "default": "0.4in"
                        },
                        "right": {
                            "type": "string",
                            "description": "Right margin (e.g., '1in', '20mm')",
                            "default": "0.4in"
                        }
                    }
                },
                "header_template": {
                    "type": "string",
                    "description": "HTML template for the header (should use <span> with specific classes: date, title, url, pageNumber, totalPages)"
                },
                "footer_template": {
                    "type": "string",
                    "description": "HTML template for the footer (same classes as header)"
                },
                "display_header_footer": {
                    "type": "boolean",
                    "description": "Whether to display header and footer",
                    "default": false
                },
                "page_ranges": {
                    "type": "string",
                    "description": "Page ranges to print (e.g., '1-5, 8, 11-13')"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        ctx: &ToolContext,
    ) -> anyhow::Result<String> {
        let args: PdfArgs = serde_json::from_value(input)?;
        
        let pages = self.browser.pages().await.map_err(|e| {
            anyhow::anyhow!("Failed to get pages: {}", e)
        })?;
        
        let page = pages.first().ok_or_else(|| {
            anyhow::anyhow!("No pages available")
        })?;
        
        // Resolve path
        let path = if args.path.starts_with('/') {
            args.path.clone()
        } else if let Some(workspace) = &ctx.workspace_path {
            format!("{}/{}", workspace, args.path)
        } else {
            args.path.clone()
        };
        
        info!(path = %path, "Generating PDF");
        
        // Build PDF options
        let mut options = PdfOptions::default();
        
        // Format or custom dimensions
        if let Some(width) = args.width {
            options.width = Some(parse_dimension(&width)?);
        }
        if let Some(height) = args.height {
            options.height = Some(parse_dimension(&height)?);
        }
        if options.width.is_none() && options.height.is_none() {
            options.format = args.format.or_else(|| Some("A4".to_string()));
        }
        
        options.print_background = args.print_background.unwrap_or(true);
        options.scale = args.scale;
        
        // Margins
        if let Some(margin) = args.margin {
            options.margin_top = margin.top.map(|s| parse_dimension(&s)).transpose()?;
            options.margin_bottom = margin.bottom.map(|s| parse_dimension(&s)).transpose()?;
            options.margin_left = margin.left.map(|s| parse_dimension(&s)).transpose()?;
            options.margin_right = margin.right.map(|s| parse_dimension(&s)).transpose()?;
        }
        
        options.display_header_footer = args.display_header_footer.unwrap_or(false);
        options.header_template = args.header_template;
        options.footer_template = args.footer_template;
        options.page_ranges = args.page_ranges;
        
        // Generate PDF
        let pdf_data = page.pdf(options).await.map_err(|e| {
            anyhow::anyhow!("Failed to generate PDF: {}", e)
        })?;
        
        // Save PDF
        std::fs::write(&path, &pdf_data).map_err(|e| {
            anyhow::anyhow!("Failed to save PDF: {}", e)
        })?;
        
        let file_size = pdf_data.len();
        let file_size_str = if file_size > 1024 * 1024 {
            format!("{:.2} MB", file_size as f64 / (1024.0 * 1024.0))
        } else if file_size > 1024 {
            format!("{:.2} KB", file_size as f64 / 1024.0)
        } else {
            format!("{} B", file_size)
        };
        
        Ok(success_response(format!(
            "PDF generated successfully\nSaved to: {}\nFile size: {}",
            path, file_size_str
        )))
    }
}

/// Parse dimension string (e.g., "8.5in", "210mm") to inches.
fn parse_dimension(s: &str) -> anyhow::Result<f64> {
    let s = s.trim();
    
    if let Some(idx) = s.find("in") {
        let num: f64 = s[..idx].parse()?;
        return Ok(num);
    }
    
    if let Some(idx) = s.find("mm") {
        let num: f64 = s[..idx].parse()?;
        return Ok(num / 25.4);
    }
    
    if let Some(idx) = s.find("cm") {
        let num: f64 = s[..idx].parse()?;
        return Ok(num / 2.54);
    }
    
    if let Some(idx) = s.find("px") {
        let num: f64 = s[..idx].parse()?;
        // Assume 96 DPI
        return Ok(num / 96.0);
    }
    
    // Assume inches if no unit
    s.parse().map_err(|e| anyhow::anyhow!("Invalid dimension '{}': {}", s, e))
}

/// Arguments for PDF tool.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PdfArgs {
    /// File path to save.
    pub path: String,
    /// Paper format.
    #[serde(default)]
    pub format: Option<String>,
    /// Custom width.
    #[serde(default)]
    pub width: Option<String>,
    /// Custom height.
    #[serde(default)]
    pub height: Option<String>,
    /// Print background.
    #[serde(default)]
    pub print_background: Option<bool>,
    /// Landscape orientation.
    #[serde(default)]
    pub landscape: Option<bool>,
    /// Scale factor.
    #[serde(default)]
    pub scale: Option<f64>,
    /// Margins.
    #[serde(default)]
    pub margin: Option<MarginArgs>,
    /// Header template.
    #[serde(default)]
    pub header_template: Option<String>,
    /// Footer template.
    #[serde(default)]
    pub footer_template: Option<String>,
    /// Display header/footer.
    #[serde(default)]
    pub display_header_footer: Option<bool>,
    /// Page ranges.
    #[serde(default)]
    pub page_ranges: Option<String>,
}

/// Margin arguments.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MarginArgs {
    /// Top margin.
    #[serde(default)]
    pub top: Option<String>,
    /// Bottom margin.
    #[serde(default)]
    pub bottom: Option<String>,
    /// Left margin.
    #[serde(default)]
    pub left: Option<String>,
    /// Right margin.
    #[serde(default)]
    pub right: Option<String>,
}
