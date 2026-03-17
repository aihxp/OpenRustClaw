//! Download file tool for browser automation.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::info;

use crate::browser::Browser;
use crate::tools::{AutomationTool, ToolContext, error_response, success_response};

/// Download a file from the browser.
pub struct DownloadTool {
    browser: Browser,
}

impl DownloadTool {
    /// Create a new download tool.
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
impl AutomationTool for DownloadTool {
    fn name(&self) -> &str {
        "browser_download"
    }

    fn description(&self) -> &str {
        "Download a file from the current page. \
         Can download by clicking a link, extracting a URL, or providing a direct URL. \
         Useful for downloading PDFs, images, documents, and other files. \
         The download will be saved to the configured download directory."
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "Direct URL to download from"
                },
                "selector": {
                    "type": "string",
                    "description": "CSS selector for a download link to click"
                },
                "filename": {
                    "type": "string",
                    "description": "Custom filename for the downloaded file (optional)"
                },
                "path": {
                    "type": "string",
                    "description": "Directory path to save the file (optional, defaults to download directory)"
                },
                "wait_for_download": {
                    "type": "boolean",
                    "description": "Whether to wait for the download to complete",
                    "default": true
                },
                "timeout": {
                    "type": "integer",
                    "description": "Maximum time to wait for download in milliseconds",
                    "default": 30000
                }
            },
            "oneOf": [
                { "required": ["url"] },
                { "required": ["selector"] }
            ]
        })
    }

    async fn execute(&self, input: serde_json::Value, ctx: &ToolContext) -> anyhow::Result<String> {
        let args: DownloadArgs = serde_json::from_value(input)?;

        let pages = self
            .browser
            .pages()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get pages: {}", e))?;

        let page = pages
            .first()
            .ok_or_else(|| anyhow::anyhow!("No pages available"))?;

        // Determine download path
        let download_dir = if let Some(path) = args.path {
            if path.starts_with('/') {
                std::path::PathBuf::from(path)
            } else if let Some(workspace) = &ctx.workspace_path {
                std::path::PathBuf::from(workspace).join(path)
            } else {
                std::path::PathBuf::from(path)
            }
        } else if let Some(download_path) = &self.browser.config().automation.download_path {
            std::path::PathBuf::from(download_path)
        } else if let Some(workspace) = &ctx.workspace_path {
            std::path::PathBuf::from(workspace).join("downloads")
        } else {
            std::path::PathBuf::from("./downloads")
        };

        // Ensure download directory exists
        tokio::fs::create_dir_all(&download_dir)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create download directory: {}", e))?;

        let url = if let Some(url) = args.url {
            // Direct URL download
            info!(url = %url, "Downloading from URL");
            url
        } else if let Some(selector) = args.selector {
            // Click to download
            info!(selector = %selector, "Clicking element to download");

            let element = page
                .query_selector(&selector)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to find element: {}", e))?;

            match element {
                Some(el) => {
                    // Get href if it's a link
                    let href = el.get_attribute("href").await.ok().flatten();

                    // Click the element
                    el.click()
                        .await
                        .map_err(|e| anyhow::anyhow!("Failed to click element: {}", e))?;

                    // If we got an href, use it
                    if let Some(href) = href {
                        if href.starts_with("http") || href.starts_with("//") {
                            href
                        } else {
                            // Relative URL - resolve it
                            let current_url = page.url().await.unwrap_or_default();
                            resolve_url(&current_url, &href)
                        }
                    } else {
                        // Wait for download to start
                        if args.wait_for_download {
                            let _timeout = args.timeout.unwrap_or(30000);
                            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                        }

                        return Ok(success_response(
                            "Download initiated by clicking element. File should be in downloads directory.".to_string()
                        ));
                    }
                }
                None => {
                    return Ok(error_response(format!("Element not found: {}", selector)));
                }
            }
        } else {
            return Ok(error_response(
                "Either 'url' or 'selector' must be provided",
            ));
        };

        // Download the file using HTTP client
        let response = reqwest::get(&url)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to download file: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            return Ok(error_response(format!(
                "Download failed with status: {} {}",
                status.as_u16(),
                status.canonical_reason().unwrap_or("Unknown")
            )));
        }

        // Determine filename
        let filename = if let Some(name) = args.filename {
            name
        } else {
            // Extract from URL or Content-Disposition header
            extract_filename(&url, response.headers())
        };

        let filepath = download_dir.join(&filename);

        // Download content
        let content = response
            .bytes()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read download content: {}", e))?;

        // Save file
        tokio::fs::write(&filepath, &content)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to save file: {}", e))?;

        let file_size = content.len();
        let file_size_str = if file_size > 1024 * 1024 {
            format!("{:.2} MB", file_size as f64 / (1024.0 * 1024.0))
        } else if file_size > 1024 {
            format!("{:.2} KB", file_size as f64 / 1024.0)
        } else {
            format!("{} B", file_size)
        };

        Ok(success_response(format!(
            "File downloaded successfully\nURL: {}\nSaved to: {}\nFilename: {}\nSize: {}",
            url,
            filepath.display(),
            filename,
            file_size_str
        )))
    }
}

/// Extract filename from URL and headers.
fn extract_filename(url: &str, headers: &reqwest::header::HeaderMap) -> String {
    // Try Content-Disposition header first
    if let Some(cd) = headers.get(reqwest::header::CONTENT_DISPOSITION)
        && let Ok(cd_str) = cd.to_str()
        && let Some(filename) = parse_content_disposition(cd_str)
    {
        return filename;
    }

    // Extract from URL path
    if let Ok(parsed) = url::Url::parse(url)
        && let Some(mut segments) = parsed.path_segments()
        && let Some(last) = segments.next_back()
        && !last.is_empty()
    {
        return sanitize_filename(last);
    }

    // Default filename
    "download".to_string()
}

/// Parse Content-Disposition header for filename.
fn parse_content_disposition(header: &str) -> Option<String> {
    // Simple parser for filename="name" or filename*=UTF-8''name
    if let Some(pos) = header.find("filename*=UTF-8'") {
        let start = pos + "filename*=UTF-8'".len();
        if let Some(end) = header[start..]
            .find('"')
            .or_else(|| header[start..].find(';'))
        {
            return Some(sanitize_filename(&header[start..start + end]));
        }
        return Some(sanitize_filename(&header[start..]));
    }

    if let Some(pos) = header.find("filename=\"") {
        let start = pos + "filename=\"".len();
        if let Some(end) = header[start..].find('"') {
            return Some(sanitize_filename(&header[start..start + end]));
        }
    }

    None
}

/// Sanitize filename for filesystem.
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect()
}

/// Resolve a relative URL against a base URL.
fn resolve_url(base: &str, relative: &str) -> String {
    if let Ok(base_url) = url::Url::parse(base)
        && let Ok(resolved) = base_url.join(relative)
    {
        return resolved.to_string();
    }
    relative.to_string()
}

/// Arguments for download tool.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DownloadArgs {
    /// URL to download.
    #[serde(default)]
    pub url: Option<String>,
    /// Element selector to click.
    #[serde(default)]
    pub selector: Option<String>,
    /// Custom filename.
    #[serde(default)]
    pub filename: Option<String>,
    /// Download directory.
    #[serde(default)]
    pub path: Option<String>,
    /// Whether to wait for download.
    #[serde(default = "default_true")]
    pub wait_for_download: bool,
    /// Timeout in milliseconds.
    #[serde(default)]
    pub timeout: Option<u64>,
}

fn default_true() -> bool {
    true
}
