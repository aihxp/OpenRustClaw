use std::collections::{HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use openrustclaw_automation::browser::{
    LoadState, PdfOptions, ScreenshotFormat, ScreenshotOptions,
};
use openrustclaw_automation::{Browser, BrowserConfig};
use regex::Regex;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;
use walkdir::WalkDir;

pub const DEFAULT_BROWSER_ROOT: &str = ".claw/browser";

fn default_extract_kind() -> String {
    "text".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserNavigateRequest {
    pub url: String,
    #[serde(default)]
    pub wait_until: Option<String>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserExtractRequest {
    pub url: String,
    #[serde(default = "default_extract_kind")]
    pub what: String,
    #[serde(default)]
    pub selector: Option<String>,
    #[serde(default)]
    pub wait_until: Option<String>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub max_results: Option<usize>,
    #[serde(default)]
    pub max_chars: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserScreenshotRequest {
    pub url: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub selector: Option<String>,
    #[serde(default)]
    pub full_page: bool,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub quality: Option<u8>,
    #[serde(default)]
    pub wait_until: Option<String>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserPdfRequest {
    pub url: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub print_background: Option<bool>,
    #[serde(default)]
    pub wait_until: Option<String>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserReadPageRequest {
    pub url: String,
    #[serde(default)]
    pub max_chars: Option<usize>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserCrawlRequest {
    pub url: String,
    #[serde(default)]
    pub max_pages: Option<usize>,
    #[serde(default)]
    pub max_chars_per_page: Option<usize>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BrowserNavigateResult {
    pub backend: String,
    pub url: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BrowserExtractResult {
    pub backend: String,
    pub url: String,
    pub title: String,
    pub what: String,
    pub data: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct BrowserScreenshotResult {
    pub backend: String,
    pub url: String,
    pub title: String,
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub format: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BrowserPdfResult {
    pub backend: String,
    pub url: String,
    pub title: String,
    pub path: String,
    pub bytes: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct BrowserReadPageResult {
    pub url: String,
    pub final_url: String,
    pub status: u16,
    pub title: String,
    pub content_type: Option<String>,
    pub text: String,
    pub link_count: usize,
    pub artifact_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserCrawlPageResult {
    pub url: String,
    pub depth: usize,
    pub title: String,
    pub link_count: usize,
    pub text_excerpt: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BrowserCrawlResult {
    pub root_url: String,
    pub page_count: usize,
    pub pages: Vec<BrowserCrawlPageResult>,
    pub artifact_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserArtifactSummary {
    pub kind: String,
    pub name: String,
    pub path: String,
    pub bytes: u64,
    #[serde(default)]
    pub modified_at: Option<String>,
}

pub async fn navigate(
    workspace_root: &Path,
    request: BrowserNavigateRequest,
) -> Result<BrowserNavigateResult> {
    let browser = new_browser(workspace_root, request.timeout_ms).await?;
    let backend = browser.config().automation.browser.to_string();
    let page = open_page(&browser, &request.url, request.wait_until.as_deref()).await?;
    let result = BrowserNavigateResult {
        backend,
        url: page
            .url()
            .await
            .unwrap_or_else(|_| normalize_url(&request.url)),
        title: page.title().await.unwrap_or_default(),
    };
    browser.close().await?;
    Ok(result)
}

pub async fn read_page(
    workspace_root: &Path,
    request: BrowserReadPageRequest,
) -> Result<BrowserReadPageResult> {
    let fetched = fetch_html(&request.url, request.timeout_ms).await?;
    let max_chars = request.max_chars.unwrap_or(4000);
    let text = truncate_string(html_to_text(&fetched.body), max_chars);
    let links = extract_links(&fetched.final_url, &fetched.body);
    let artifact_path =
        resolve_output_path(workspace_root, request.path.as_deref(), "read", "json")?;
    let payload = serde_json::json!({
        "url": fetched.requested_url,
        "final_url": fetched.final_url,
        "status": fetched.status,
        "title": fetched.title,
        "content_type": fetched.content_type,
        "text": text,
        "links": links,
    });
    fs::write(&artifact_path, serde_json::to_vec_pretty(&payload)?)
        .with_context(|| format!("Failed to write '{}'", artifact_path.display()))?;
    Ok(BrowserReadPageResult {
        url: fetched.requested_url,
        final_url: fetched.final_url,
        status: fetched.status,
        title: fetched.title,
        content_type: fetched.content_type,
        text,
        link_count: links.len(),
        artifact_path: artifact_path.display().to_string(),
    })
}

pub async fn crawl_site(
    workspace_root: &Path,
    request: BrowserCrawlRequest,
) -> Result<BrowserCrawlResult> {
    let max_pages = request.max_pages.unwrap_or(5).max(1);
    let max_chars_per_page = request.max_chars_per_page.unwrap_or(1200);
    let root_url = normalize_url(&request.url);
    let root_parsed = Url::parse(&root_url).context("invalid crawl root url")?;
    let mut queue = VecDeque::from([(root_url.clone(), 0usize)]);
    let mut seen = HashSet::from([root_url.clone()]);
    let mut pages = Vec::new();

    while let Some((url, depth)) = queue.pop_front() {
        if pages.len() >= max_pages {
            break;
        }
        let fetched = fetch_html(&url, request.timeout_ms).await?;
        let text = truncate_string(html_to_text(&fetched.body), max_chars_per_page);
        let links = extract_links(&fetched.final_url, &fetched.body);
        pages.push(BrowserCrawlPageResult {
            url: fetched.final_url.clone(),
            depth,
            title: fetched.title,
            link_count: links.len(),
            text_excerpt: text,
        });
        for link in links {
            if pages.len() + queue.len() >= max_pages {
                break;
            }
            let Ok(parsed) = Url::parse(&link) else {
                continue;
            };
            if parsed.domain() != root_parsed.domain() {
                continue;
            }
            let candidate = parsed.to_string();
            if seen.insert(candidate.clone()) {
                queue.push_back((candidate, depth + 1));
            }
        }
    }

    let artifact_path =
        resolve_output_path(workspace_root, request.path.as_deref(), "crawl", "json")?;
    let payload = serde_json::json!({
        "root_url": root_url,
        "page_count": pages.len(),
        "pages": pages,
    });
    fs::write(&artifact_path, serde_json::to_vec_pretty(&payload)?)
        .with_context(|| format!("Failed to write '{}'", artifact_path.display()))?;
    let pages: Vec<BrowserCrawlPageResult> = serde_json::from_value(payload["pages"].clone())?;
    Ok(BrowserCrawlResult {
        root_url,
        page_count: pages.len(),
        pages,
        artifact_path: artifact_path.display().to_string(),
    })
}

pub async fn extract(
    workspace_root: &Path,
    request: BrowserExtractRequest,
) -> Result<BrowserExtractResult> {
    let browser = new_browser(workspace_root, request.timeout_ms).await?;
    let backend = browser.config().automation.browser.to_string();
    let page = open_page(&browser, &request.url, request.wait_until.as_deref()).await?;
    let title = page.title().await.unwrap_or_default();
    let url = page
        .url()
        .await
        .unwrap_or_else(|_| normalize_url(&request.url));
    let what = request.what.trim().to_lowercase();
    let max_results = request.max_results.unwrap_or(100);
    let max_chars = request.max_chars.unwrap_or(4000);

    let data = match what.as_str() {
        "html" => Value::String(truncate_string(page.content().await?, max_chars)),
        "links" => {
            let value = page
                .evaluate(
                    r#"
                    Array.from(document.querySelectorAll('a[href]')).map(a => ({
                        text: (a.textContent || '').trim(),
                        href: a.href,
                        title: a.title || ''
                    }))
                    "#,
                )
                .await?;
            limit_value_array(value, max_results)
        }
        "images" => {
            let value = page
                .evaluate(
                    r#"
                    Array.from(document.querySelectorAll('img')).map(img => ({
                        src: img.src,
                        alt: img.alt || '',
                        width: img.naturalWidth || null,
                        height: img.naturalHeight || null
                    })).filter(img => img.src)
                    "#,
                )
                .await?;
            limit_value_array(value, max_results)
        }
        "headings" => {
            let value = page
                .evaluate(
                    r#"
                    Array.from(document.querySelectorAll('h1, h2, h3, h4, h5, h6')).map(node => ({
                        level: Number(node.tagName.substring(1)),
                        text: (node.textContent || '').trim()
                    }))
                    "#,
                )
                .await?;
            limit_value_array(value, max_results)
        }
        "selector" => {
            let selector = request
                .selector
                .as_deref()
                .context("selector is required when what=selector")?;
            let script = format!(
                r#"
                Array.from(document.querySelectorAll({selector:?})).map(node => ({{
                    tag: node.tagName.toLowerCase(),
                    text: (node.textContent || '').trim(),
                    html: node.innerHTML || ''
                }}))
                "#
            );
            let value = page.evaluate(&script).await?;
            limit_value_array(value, max_results)
        }
        _ => {
            let value = page
                .evaluate(
                    r#"
                    (function() {
                        const walker = document.createTreeWalker(
                            document.body,
                            NodeFilter.SHOW_TEXT,
                            null,
                            false
                        );
                        let text = '';
                        let node;
                        while ((node = walker.nextNode())) {
                            const parent = node.parentElement;
                            if (parent && getComputedStyle(parent).display !== 'none') {
                                text += (node.textContent || '') + ' ';
                            }
                        }
                        return text.trim().replace(/\s+/g, ' ');
                    })()
                    "#,
                )
                .await?;
            Value::String(truncate_string(
                value.as_str().unwrap_or_default().to_string(),
                max_chars,
            ))
        }
    };

    browser.close().await?;
    Ok(BrowserExtractResult {
        backend,
        url,
        title,
        what,
        data,
    })
}

pub async fn screenshot(
    workspace_root: &Path,
    request: BrowserScreenshotRequest,
) -> Result<BrowserScreenshotResult> {
    let browser = new_browser(workspace_root, request.timeout_ms).await?;
    let backend = browser.config().automation.browser.to_string();
    let page = open_page(&browser, &request.url, request.wait_until.as_deref()).await?;
    let title = page.title().await.unwrap_or_default();
    let url = page
        .url()
        .await
        .unwrap_or_else(|_| normalize_url(&request.url));

    let screenshot = if let Some(selector) = request.selector.as_deref() {
        let element = page
            .query_selector(selector)
            .await?
            .with_context(|| format!("Element not found for selector '{}'", selector))?;
        element.screenshot().await?
    } else {
        let format = parse_screenshot_format(request.format.as_deref());
        page.screenshot_with_options(ScreenshotOptions {
            format,
            quality: request.quality.map(|value| value.min(100)),
            clip: None,
            full_page: request.full_page,
            hide_selectors: vec![],
        })
        .await?
    };

    let resolved_path = resolve_output_path(
        workspace_root,
        request.path.as_deref(),
        "screenshots",
        match screenshot.format {
            ScreenshotFormat::Png => "png",
            ScreenshotFormat::Jpeg => "jpg",
        },
    )?;
    screenshot.save(&resolved_path.to_string_lossy())?;
    browser.close().await?;

    Ok(BrowserScreenshotResult {
        backend,
        url,
        title,
        path: resolved_path.display().to_string(),
        width: screenshot.width,
        height: screenshot.height,
        format: match screenshot.format {
            ScreenshotFormat::Png => "png".to_string(),
            ScreenshotFormat::Jpeg => "jpeg".to_string(),
        },
    })
}

pub async fn pdf(workspace_root: &Path, request: BrowserPdfRequest) -> Result<BrowserPdfResult> {
    let browser = new_browser(workspace_root, request.timeout_ms).await?;
    let backend = browser.config().automation.browser.to_string();
    let page = open_page(&browser, &request.url, request.wait_until.as_deref()).await?;
    let title = page.title().await.unwrap_or_default();
    let url = page
        .url()
        .await
        .unwrap_or_else(|_| normalize_url(&request.url));
    let path = resolve_output_path(workspace_root, request.path.as_deref(), "pdf", "pdf")?;
    let pdf = page
        .pdf(PdfOptions {
            format: request.format.or_else(|| Some("A4".to_string())),
            print_background: request.print_background.unwrap_or(true),
            ..Default::default()
        })
        .await?;
    fs::write(&path, &pdf).with_context(|| format!("Failed to write '{}'", path.display()))?;
    browser.close().await?;

    Ok(BrowserPdfResult {
        backend,
        url,
        title,
        path: path.display().to_string(),
        bytes: pdf.len(),
    })
}

pub fn list_artifacts(workspace_root: &Path, limit: usize) -> Result<Vec<BrowserArtifactSummary>> {
    let root = browser_root_for(workspace_root);
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut artifacts = Vec::new();
    for entry in WalkDir::new(&root)
        .min_depth(1)
        .max_depth(3)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
    {
        let metadata = entry.metadata()?;
        let relative = entry
            .path()
            .strip_prefix(workspace_root)
            .unwrap_or(entry.path())
            .display()
            .to_string();
        let kind = entry
            .path()
            .parent()
            .and_then(|parent| parent.file_name())
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_else(|| "browser".to_string());
        artifacts.push(BrowserArtifactSummary {
            kind,
            name: entry.file_name().to_string_lossy().to_string(),
            path: relative,
            bytes: metadata.len(),
            modified_at: metadata
                .modified()
                .ok()
                .map(|timestamp| chrono::DateTime::<chrono::Utc>::from(timestamp).to_rfc3339()),
        });
    }

    artifacts.sort_by(|left, right| right.modified_at.cmp(&left.modified_at));
    artifacts.truncate(limit);
    Ok(artifacts)
}

async fn new_browser(workspace_root: &Path, timeout_ms: Option<u64>) -> Result<Browser> {
    let mut config = BrowserConfig::default();
    config.automation.timeout_ms = timeout_ms.unwrap_or(config.automation.timeout_ms);
    let downloads = browser_root_for(workspace_root).join("downloads");
    fs::create_dir_all(&downloads)
        .with_context(|| format!("Failed to create '{}'", downloads.display()))?;
    config.automation.download_path = Some(downloads.display().to_string());
    Browser::new(config).await.map_err(Into::into)
}

async fn open_page(
    browser: &Browser,
    url: &str,
    wait_until: Option<&str>,
) -> Result<openrustclaw_automation::Page> {
    let page = browser.new_page().await?;
    page.goto(&normalize_url(url)).await?;
    page.wait_for_load_state(parse_load_state(wait_until))
        .await?;
    Ok(page)
}

fn parse_load_state(value: Option<&str>) -> LoadState {
    match value.unwrap_or("load").to_ascii_lowercase().as_str() {
        "domcontentloaded" => LoadState::DomContentLoaded,
        "networkidle" => LoadState::NetworkIdle,
        _ => LoadState::Load,
    }
}

fn parse_screenshot_format(value: Option<&str>) -> ScreenshotFormat {
    match value.unwrap_or("png").to_ascii_lowercase().as_str() {
        "jpeg" | "jpg" => ScreenshotFormat::Jpeg,
        _ => ScreenshotFormat::Png,
    }
}

fn normalize_url(url: &str) -> String {
    if url.starts_with("http://") || url.starts_with("https://") {
        url.to_string()
    } else {
        format!("https://{}", url)
    }
}

fn browser_root_for(workspace_root: &Path) -> PathBuf {
    workspace_root.join(DEFAULT_BROWSER_ROOT)
}

fn resolve_output_path(
    workspace_root: &Path,
    explicit: Option<&str>,
    category: &str,
    extension: &str,
) -> Result<PathBuf> {
    let path = if let Some(explicit) = explicit {
        let explicit_path = PathBuf::from(explicit);
        if explicit_path.is_absolute() {
            explicit_path
        } else {
            workspace_root.join(explicit_path)
        }
    } else {
        browser_root_for(workspace_root)
            .join(category)
            .join(format!("{}.{}", Uuid::new_v4(), extension))
    };

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create '{}'", parent.display()))?;
    }
    Ok(path)
}

fn truncate_string(value: String, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

fn limit_value_array(value: Value, max_results: usize) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.into_iter().take(max_results).collect()),
        other => other,
    }
}

struct FetchedPage {
    requested_url: String,
    final_url: String,
    status: u16,
    title: String,
    content_type: Option<String>,
    body: String,
}

async fn fetch_html(url: &str, timeout_ms: Option<u64>) -> Result<FetchedPage> {
    let normalized = normalize_url(url);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(
            timeout_ms.unwrap_or(15_000),
        ))
        .user_agent("OpenRustClaw/Phase6Browser")
        .build()?;
    let response = client.get(&normalized).send().await?;
    let final_url = response.url().to_string();
    let status = response.status().as_u16();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(ToString::to_string);
    let body = response.text().await?;
    Ok(FetchedPage {
        requested_url: normalized,
        final_url,
        status,
        title: extract_title(&body),
        content_type,
        body,
    })
}

fn extract_title(html: &str) -> String {
    let re = Regex::new("(?is)<title[^>]*>(.*?)</title>").expect("valid title regex");
    re.captures(html)
        .and_then(|caps| caps.get(1))
        .map(|m| html_decode_minimal(m.as_str()).trim().to_string())
        .unwrap_or_default()
}

fn html_to_text(html: &str) -> String {
    let scripts = Regex::new("(?is)<script[^>]*>.*?</script>|<style[^>]*>.*?</style>")
        .expect("valid script/style regex");
    let tags = Regex::new("(?is)<[^>]+>").expect("valid tag regex");
    let whitespace = Regex::new(r"\s+").expect("valid whitespace regex");
    let without_scripts = scripts.replace_all(html, " ");
    let without_tags = tags.replace_all(&without_scripts, " ");
    html_decode_minimal(&whitespace.replace_all(&without_tags, " "))
        .trim()
        .to_string()
}

fn extract_links(base_url: &str, html: &str) -> Vec<String> {
    let Ok(base) = Url::parse(base_url) else {
        return Vec::new();
    };
    let href_re = Regex::new(r#"(?is)href\s*=\s*["']([^"'#]+)["']"#).expect("valid href regex");
    let mut links = Vec::new();
    let mut seen = HashSet::new();
    for capture in href_re.captures_iter(html) {
        let Some(raw) = capture.get(1).map(|m| m.as_str().trim()) else {
            continue;
        };
        if raw.is_empty() || raw.starts_with("javascript:") || raw.starts_with("mailto:") {
            continue;
        }
        let Ok(joined) = base.join(raw) else {
            continue;
        };
        let value = joined.to_string();
        if seen.insert(value.clone()) {
            links.push(value);
        }
    }
    links
}

fn html_decode_minimal(input: &str) -> String {
    input
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

#[cfg(test)]
mod tests {
    use super::{
        extract_links, extract_title, html_to_text, normalize_url, parse_load_state,
        resolve_output_path,
    };
    use openrustclaw_automation::browser::LoadState;
    use tempfile::tempdir;

    #[test]
    fn normalize_url_adds_https() {
        assert_eq!(normalize_url("example.com"), "https://example.com");
        assert_eq!(normalize_url("https://example.com"), "https://example.com");
    }

    #[test]
    fn parse_load_state_defaults_to_load() {
        assert!(matches!(parse_load_state(None), LoadState::Load));
        assert!(matches!(
            parse_load_state(Some("networkidle")),
            LoadState::NetworkIdle
        ));
    }

    #[test]
    fn resolve_output_path_uses_workspace_relative_default() {
        let root = tempdir().unwrap();
        let path = resolve_output_path(root.path(), None, "screenshots", "png").unwrap();
        assert!(path.starts_with(root.path()));
        assert!(path.extension().is_some_and(|ext| ext == "png"));
    }

    #[test]
    fn list_artifacts_reads_workspace_browser_outputs() {
        let root = tempdir().unwrap();
        let output = resolve_output_path(root.path(), None, "screenshots", "png").unwrap();
        std::fs::write(&output, b"hello").unwrap();
        let artifacts = super::list_artifacts(root.path(), 10).unwrap();
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].kind, "screenshots");
        assert_eq!(artifacts[0].bytes, 5);
    }

    #[test]
    fn html_helpers_extract_title_text_and_links() {
        let html = r#"
            <html>
              <head><title>Example &amp; Test</title></head>
              <body>
                <h1>Hello</h1>
                <p>World</p>
                <a href="/docs">Docs</a>
                <a href="https://example.com/about">About</a>
              </body>
            </html>
        "#;
        assert_eq!(extract_title(html), "Example & Test");
        assert!(html_to_text(html).contains("Hello World"));
        let links = extract_links("https://example.com/start", html);
        assert_eq!(links.len(), 2);
        assert!(links.contains(&"https://example.com/docs".to_string()));
        assert!(links.contains(&"https://example.com/about".to_string()));
    }
}
