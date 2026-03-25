use std::collections::{BTreeMap, HashSet, VecDeque};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use openrustclaw_automation::browser::{
    Cookie, LoadState, PdfOptions, ScreenshotFormat, ScreenshotOptions,
};
use openrustclaw_automation::{Browser, BrowserConfig, Page};
use openrustclaw_core::config::AppConfig;
use regex::Regex;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::process::Command;
use tokio::time::{Duration, timeout};
use uuid::Uuid;
use walkdir::WalkDir;

pub const DEFAULT_BROWSER_ROOT: &str = ".claw/browser";
fn default_extract_kind() -> String {
    "text".to_string()
}

fn default_browser_inspect_kind() -> String {
    "snapshot".to_string()
}

fn default_browser_backend() -> String {
    "native_cdp".to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BrowserBackendKind {
    NativeCdp,
    AgentBrowserCli,
}

impl BrowserBackendKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::NativeCdp => "native_cdp",
            Self::AgentBrowserCli => "agent_browser_cli",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternalBackendPolicy {
    pub allowed_backends: Vec<String>,
    pub allow_local_cli_wrappers: bool,
    pub allow_cloud_agent_execution: bool,
    pub audit_log_path: String,
    pub command_env_allowlist: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalBackendAuditEntry {
    pub timestamp: String,
    pub backend: String,
    pub transport: String,
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    pub allowed: bool,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserOpenSessionRequest {
    #[serde(default)]
    pub backend: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserInspectRequest {
    pub url: String,
    #[serde(default = "default_browser_backend")]
    pub backend: String,
    #[serde(default = "default_browser_inspect_kind")]
    pub kind: String,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub selector: Option<String>,
    #[serde(default)]
    pub interactive_only: bool,
    #[serde(default)]
    pub snapshot_depth: Option<usize>,
    #[serde(default)]
    pub wait_until: Option<String>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserRunSequenceRequest {
    #[serde(default = "default_browser_backend")]
    pub backend: String,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub steps: Vec<BrowserActionStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserActionStep {
    pub action: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub selector: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub values: Option<Vec<String>>,
    #[serde(default)]
    pub wait_ms: Option<u64>,
    #[serde(default)]
    pub what: Option<String>,
    #[serde(default)]
    pub max_chars: Option<usize>,
    #[serde(default)]
    pub wait_until: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub full_page: Option<bool>,
    #[serde(default)]
    pub storage_key: Option<String>,
    #[serde(default)]
    pub storage_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserNavigateRequest {
    pub url: String,
    #[serde(default = "default_browser_backend")]
    pub backend: String,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub wait_until: Option<String>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserExtractRequest {
    pub url: String,
    #[serde(default = "default_browser_backend")]
    pub backend: String,
    #[serde(default)]
    pub session_id: Option<String>,
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
    #[serde(default = "default_browser_backend")]
    pub backend: String,
    #[serde(default)]
    pub session_id: Option<String>,
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
    #[serde(default = "default_browser_backend")]
    pub backend: String,
    #[serde(default)]
    pub session_id: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserSessionSummary {
    pub id: String,
    #[serde(default)]
    pub label: Option<String>,
    pub backend: String,
    pub created_at: String,
    pub updated_at: String,
    pub state_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BrowserInspectResult {
    pub backend: String,
    pub session_id: Option<String>,
    pub url: String,
    pub title: String,
    pub kind: String,
    pub data: Value,
    pub artifact_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BrowserActionStepResult {
    pub index: usize,
    pub action: String,
    pub ok: bool,
    pub detail: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct BrowserRunSequenceResult {
    pub backend: String,
    pub session_id: Option<String>,
    pub final_url: String,
    pub title: String,
    pub steps: Vec<BrowserActionStepResult>,
    pub artifact_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BrowserSessionRecord {
    pub id: String,
    #[serde(default)]
    pub label: Option<String>,
    pub backend: String,
    pub created_at: String,
    pub updated_at: String,
    pub state_path: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct BrowserSessionState {
    #[serde(default)]
    pub cookies: Vec<Cookie>,
    #[serde(default)]
    pub local_storage: BTreeMap<String, String>,
    #[serde(default)]
    pub session_storage: BTreeMap<String, String>,
}

pub async fn navigate(
    workspace_root: &Path,
    request: BrowserNavigateRequest,
) -> Result<BrowserNavigateResult> {
    let backend = resolve_browser_backend(
        workspace_root,
        Some(request.backend.as_str()),
        "navigate",
        request.session_id.as_deref(),
    )?;
    let session = load_session_for_request(workspace_root, request.session_id.as_deref())?;
    match backend {
        BrowserBackendKind::NativeCdp => {
            let browser = new_browser(workspace_root, request.timeout_ms).await?;
            let session_state = load_session_state_for_record(workspace_root, session.as_ref())?;
            let page = open_page(
                &browser,
                &request.url,
                request.wait_until.as_deref(),
                session_state.as_ref(),
            )
            .await?;
            let result = BrowserNavigateResult {
                backend: backend.as_str().to_string(),
                url: page
                    .url()
                    .await
                    .unwrap_or_else(|_| normalize_url(&request.url)),
                title: page.title().await.unwrap_or_default(),
            };
            if let Some(record) = session.as_ref() {
                persist_page_session_state(workspace_root, record, &page, session_state.as_ref())
                    .await?;
            }
            browser.close().await?;
            Ok(result)
        }
        BrowserBackendKind::AgentBrowserCli => {
            let normalized = normalize_url(&request.url);
            let output = agent_browser_open_url(
                workspace_root,
                session.as_ref(),
                request.timeout_ms,
                &normalized,
            )
            .await?;
            Ok(BrowserNavigateResult {
                backend: backend.as_str().to_string(),
                url: output
                    .get("data")
                    .and_then(|data| data.get("url"))
                    .and_then(Value::as_str)
                    .unwrap_or(&request.url)
                    .to_string(),
                title: output
                    .get("data")
                    .and_then(|data| data.get("title"))
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
            })
        }
    }
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
    let backend = resolve_browser_backend(
        workspace_root,
        Some(request.backend.as_str()),
        "extract",
        request.session_id.as_deref(),
    )?;
    let session = load_session_for_request(workspace_root, request.session_id.as_deref())?;
    match backend {
        BrowserBackendKind::NativeCdp => {
            let browser = new_browser(workspace_root, request.timeout_ms).await?;
            let session_state = load_session_state_for_record(workspace_root, session.as_ref())?;
            let page = open_page(
                &browser,
                &request.url,
                request.wait_until.as_deref(),
                session_state.as_ref(),
            )
            .await?;
            let title = page.title().await.unwrap_or_default();
            let url = page
                .url()
                .await
                .unwrap_or_else(|_| normalize_url(&request.url));
            let what = request.what.trim().to_lowercase();
            let data = extract_from_page(&page, &request).await?;
            if let Some(record) = session.as_ref() {
                persist_page_session_state(workspace_root, record, &page, session_state.as_ref())
                    .await?;
            }
            browser.close().await?;
            Ok(BrowserExtractResult {
                backend: backend.as_str().to_string(),
                url,
                title,
                what,
                data,
            })
        }
        BrowserBackendKind::AgentBrowserCli => {
            let normalized = normalize_url(&request.url);
            agent_browser_open_url(
                workspace_root,
                session.as_ref(),
                request.timeout_ms,
                &normalized,
            )
            .await?;
            let data = agent_browser_extract(
                workspace_root,
                session.as_ref(),
                request.timeout_ms,
                &request,
            )
            .await?;
            let title = agent_browser_get(
                workspace_root,
                session.as_ref(),
                request.timeout_ms,
                "title",
                None,
            )
            .await?;
            let url = agent_browser_get(
                workspace_root,
                session.as_ref(),
                request.timeout_ms,
                "url",
                None,
            )
            .await?
            .as_str()
            .unwrap_or(&normalized)
            .to_string();
            Ok(BrowserExtractResult {
                backend: backend.as_str().to_string(),
                url,
                title: title.as_str().unwrap_or_default().to_string(),
                what: request.what.trim().to_lowercase(),
                data,
            })
        }
    }
}

pub async fn screenshot(
    workspace_root: &Path,
    request: BrowserScreenshotRequest,
) -> Result<BrowserScreenshotResult> {
    let backend = resolve_browser_backend(
        workspace_root,
        Some(request.backend.as_str()),
        "screenshot",
        request.session_id.as_deref(),
    )?;
    let session = load_session_for_request(workspace_root, request.session_id.as_deref())?;
    match backend {
        BrowserBackendKind::NativeCdp => {
            let browser = new_browser(workspace_root, request.timeout_ms).await?;
            let session_state = load_session_state_for_record(workspace_root, session.as_ref())?;
            let page = open_page(
                &browser,
                &request.url,
                request.wait_until.as_deref(),
                session_state.as_ref(),
            )
            .await?;
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
            if let Some(record) = session.as_ref() {
                persist_page_session_state(workspace_root, record, &page, session_state.as_ref())
                    .await?;
            }
            browser.close().await?;

            Ok(BrowserScreenshotResult {
                backend: backend.as_str().to_string(),
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
        BrowserBackendKind::AgentBrowserCli => {
            let normalized = normalize_url(&request.url);
            agent_browser_open_url(
                workspace_root,
                session.as_ref(),
                request.timeout_ms,
                &normalized,
            )
            .await?;
            let resolved_path = resolve_output_path(
                workspace_root,
                request.path.as_deref(),
                "screenshots",
                request
                    .format
                    .as_deref()
                    .map(|value| {
                        if value.eq_ignore_ascii_case("jpeg") || value.eq_ignore_ascii_case("jpg") {
                            "jpg"
                        } else {
                            "png"
                        }
                    })
                    .unwrap_or("png"),
            )?;
            let mut args = vec![
                "screenshot".to_string(),
                resolved_path.display().to_string(),
            ];
            if request.full_page {
                args.push("--full".to_string());
            }
            run_agent_browser_command(workspace_root, session.as_ref(), request.timeout_ms, &args)
                .await?;
            Ok(BrowserScreenshotResult {
                backend: backend.as_str().to_string(),
                url: normalized,
                title: agent_browser_get(
                    workspace_root,
                    session.as_ref(),
                    request.timeout_ms,
                    "title",
                    None,
                )
                .await?
                .as_str()
                .unwrap_or_default()
                .to_string(),
                path: resolved_path.display().to_string(),
                width: 0,
                height: 0,
                format: request.format.unwrap_or_else(|| "png".to_string()),
            })
        }
    }
}

pub async fn pdf(workspace_root: &Path, request: BrowserPdfRequest) -> Result<BrowserPdfResult> {
    let backend = resolve_browser_backend(
        workspace_root,
        Some(request.backend.as_str()),
        "pdf",
        request.session_id.as_deref(),
    )?;
    let session = load_session_for_request(workspace_root, request.session_id.as_deref())?;
    match backend {
        BrowserBackendKind::NativeCdp => {
            let browser = new_browser(workspace_root, request.timeout_ms).await?;
            let session_state = load_session_state_for_record(workspace_root, session.as_ref())?;
            let page = open_page(
                &browser,
                &request.url,
                request.wait_until.as_deref(),
                session_state.as_ref(),
            )
            .await?;
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
            fs::write(&path, &pdf)
                .with_context(|| format!("Failed to write '{}'", path.display()))?;
            if let Some(record) = session.as_ref() {
                persist_page_session_state(workspace_root, record, &page, session_state.as_ref())
                    .await?;
            }
            browser.close().await?;

            Ok(BrowserPdfResult {
                backend: backend.as_str().to_string(),
                url,
                title,
                path: path.display().to_string(),
                bytes: pdf.len(),
            })
        }
        BrowserBackendKind::AgentBrowserCli => {
            let normalized = normalize_url(&request.url);
            agent_browser_open_url(
                workspace_root,
                session.as_ref(),
                request.timeout_ms,
                &normalized,
            )
            .await?;
            let path = resolve_output_path(workspace_root, request.path.as_deref(), "pdf", "pdf")?;
            run_agent_browser_command(
                workspace_root,
                session.as_ref(),
                request.timeout_ms,
                &["pdf".to_string(), path.display().to_string()],
            )
            .await?;
            let bytes = fs::metadata(&path)
                .with_context(|| format!("Failed to read '{}'", path.display()))?
                .len() as usize;
            Ok(BrowserPdfResult {
                backend: backend.as_str().to_string(),
                url: normalized,
                title: agent_browser_get(
                    workspace_root,
                    session.as_ref(),
                    request.timeout_ms,
                    "title",
                    None,
                )
                .await?
                .as_str()
                .unwrap_or_default()
                .to_string(),
                path: path.display().to_string(),
                bytes,
            })
        }
    }
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

pub fn open_session(
    workspace_root: &Path,
    request: BrowserOpenSessionRequest,
) -> Result<BrowserSessionSummary> {
    let backend = resolve_browser_backend(
        workspace_root,
        Some(request.backend.as_deref().unwrap_or("native_cdp")),
        "open_session",
        request.session_id.as_deref(),
    )?;
    let session_id = request
        .session_id
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let state_path = browser_state_root_for(workspace_root).join(format!("{session_id}.json"));
    let now = chrono::Utc::now().to_rfc3339();
    let record = BrowserSessionRecord {
        id: session_id.clone(),
        label: request.label,
        backend: backend.as_str().to_string(),
        created_at: now.clone(),
        updated_at: now,
        state_path: workspace_relative_path(workspace_root, &state_path),
    };
    save_session_state(&state_path, &BrowserSessionState::default())?;
    save_session_record(workspace_root, &record)?;
    Ok(record_to_summary(record))
}

pub fn list_sessions(workspace_root: &Path, limit: usize) -> Result<Vec<BrowserSessionSummary>> {
    let root = browser_sessions_root_for(workspace_root);
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut sessions = Vec::new();
    for entry in WalkDir::new(&root)
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
    {
        let raw = fs::read_to_string(entry.path())
            .with_context(|| format!("Failed to read '{}'", entry.path().display()))?;
        let record: BrowserSessionRecord = serde_json::from_str(&raw)
            .with_context(|| format!("Failed to parse '{}'", entry.path().display()))?;
        sessions.push(record_to_summary(record));
    }
    sessions.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    sessions.truncate(limit);
    Ok(sessions)
}

pub async fn inspect(
    workspace_root: &Path,
    request: BrowserInspectRequest,
) -> Result<BrowserInspectResult> {
    let backend = resolve_browser_backend(
        workspace_root,
        Some(request.backend.as_str()),
        "inspect",
        request.session_id.as_deref(),
    )?;
    let session = load_session_for_request(workspace_root, request.session_id.as_deref())?;
    let result = match backend {
        BrowserBackendKind::NativeCdp => {
            inspect_native(workspace_root, &request, session.as_ref()).await?
        }
        BrowserBackendKind::AgentBrowserCli => {
            inspect_agent_browser(workspace_root, &request, session.as_ref()).await?
        }
    };
    Ok(result)
}

pub async fn run_sequence(
    workspace_root: &Path,
    request: BrowserRunSequenceRequest,
) -> Result<BrowserRunSequenceResult> {
    if request.steps.is_empty() {
        bail!("browser action sequence requires at least one step");
    }
    let backend = resolve_browser_backend(
        workspace_root,
        Some(request.backend.as_str()),
        "run_sequence",
        request.session_id.as_deref(),
    )?;
    let session = load_session_for_request(workspace_root, request.session_id.as_deref())?;
    match backend {
        BrowserBackendKind::NativeCdp => {
            run_native_sequence(workspace_root, &request, session.as_ref()).await
        }
        BrowserBackendKind::AgentBrowserCli => {
            run_agent_browser_sequence(workspace_root, &request, session.as_ref()).await
        }
    }
}

pub fn backend_policy(workspace_root: &Path) -> Result<ExternalBackendPolicy> {
    Ok(policy_from_config(
        &load_runtime_config_for(workspace_root),
        workspace_root,
    ))
}

pub fn list_backend_audit(
    workspace_root: &Path,
    limit: usize,
) -> Result<Vec<ExternalBackendAuditEntry>> {
    let policy = backend_policy(workspace_root)?;
    let path = external_backend_audit_path(workspace_root, &policy);
    if !path.exists() {
        return Ok(Vec::new());
    }

    let raw = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read '{}'", path.display()))?;
    let mut entries = Vec::new();
    for (index, line) in raw.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let entry: ExternalBackendAuditEntry =
            serde_json::from_str(trimmed).with_context(|| {
                format!(
                    "Failed to parse audit entry {} in '{}'",
                    index + 1,
                    path.display()
                )
            })?;
        entries.push(entry);
    }
    entries.sort_by(|left, right| right.timestamp.cmp(&left.timestamp));
    entries.truncate(limit);
    Ok(entries)
}

fn normalize_backend_name(raw: &str) -> String {
    raw.trim().to_ascii_lowercase().replace('-', "_")
}

fn load_runtime_config_for(workspace_root: &Path) -> AppConfig {
    let explicit = workspace_root.join("config/default.toml");
    if explicit.exists() {
        AppConfig::load_from(&explicit.display().to_string())
            .or_else(|_| AppConfig::load())
            .unwrap_or_default()
    } else {
        AppConfig::load().unwrap_or_default()
    }
}

fn policy_from_config(config: &AppConfig, workspace_root: &Path) -> ExternalBackendPolicy {
    let mut allowed_backends: Vec<String> = config
        .external_backends
        .allowed_backends
        .iter()
        .map(|value| normalize_backend_name(value))
        .collect();
    allowed_backends.sort();
    allowed_backends.dedup();
    let audit_path =
        absolute_workspace_path(workspace_root, &config.external_backends.audit_log_path);
    ExternalBackendPolicy {
        allowed_backends,
        allow_local_cli_wrappers: config.external_backends.allow_local_cli_wrappers,
        allow_cloud_agent_execution: config.external_backends.allow_cloud_agent_execution,
        audit_log_path: workspace_relative_path(workspace_root, &audit_path),
        command_env_allowlist: config.external_backends.command_env_allowlist.clone(),
    }
}

fn external_backend_audit_path(workspace_root: &Path, policy: &ExternalBackendPolicy) -> PathBuf {
    absolute_workspace_path(workspace_root, &policy.audit_log_path)
}

fn append_external_backend_audit_entry(
    workspace_root: &Path,
    policy: &ExternalBackendPolicy,
    entry: &ExternalBackendAuditEntry,
) -> Result<()> {
    let path = external_backend_audit_path(workspace_root, policy);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create '{}'", parent.display()))?;
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("Failed to open '{}'", path.display()))?;
    writeln!(file, "{}", serde_json::to_string(entry)?)
        .with_context(|| format!("Failed to append '{}'", path.display()))?;
    Ok(())
}

fn record_external_backend_audit(
    workspace_root: &Path,
    policy: &ExternalBackendPolicy,
    backend: BrowserBackendKind,
    action: &str,
    session_id: Option<&str>,
    allowed: bool,
    success: bool,
    detail: Option<String>,
) -> Result<()> {
    append_external_backend_audit_entry(
        workspace_root,
        policy,
        &ExternalBackendAuditEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            backend: backend.as_str().to_string(),
            transport: match backend {
                BrowserBackendKind::NativeCdp => "native".to_string(),
                BrowserBackendKind::AgentBrowserCli => "local_cli_wrapper".to_string(),
            },
            action: action.to_string(),
            session_id: session_id.map(ToString::to_string),
            allowed,
            success,
            detail,
        },
    )
}

fn ensure_backend_execution_allowed_with_policy(
    workspace_root: &Path,
    policy: &ExternalBackendPolicy,
    backend: BrowserBackendKind,
    action: &str,
    session_id: Option<&str>,
) -> Result<()> {
    if backend != BrowserBackendKind::AgentBrowserCli {
        return Ok(());
    }
    if !policy.allow_local_cli_wrappers {
        record_external_backend_audit(
            workspace_root,
            policy,
            backend,
            action,
            session_id,
            false,
            false,
            Some("local CLI wrapper execution is disabled by policy".to_string()),
        )?;
        bail!("agent-browser backend is disabled by external backend policy");
    }
    if !policy
        .allowed_backends
        .iter()
        .any(|value| value == backend.as_str())
    {
        record_external_backend_audit(
            workspace_root,
            policy,
            backend,
            action,
            session_id,
            false,
            false,
            Some("backend is not in the external backend allowlist".to_string()),
        )?;
        bail!("agent-browser backend is not in the external backend allowlist");
    }
    Ok(())
}

fn resolve_browser_backend(
    workspace_root: &Path,
    raw: Option<&str>,
    action: &str,
    session_id: Option<&str>,
) -> Result<BrowserBackendKind> {
    let backend = parse_browser_backend(raw)?;
    let policy = backend_policy(workspace_root)?;
    ensure_backend_execution_allowed_with_policy(
        workspace_root,
        &policy,
        backend,
        action,
        session_id,
    )?;
    Ok(backend)
}

fn parse_browser_backend(raw: Option<&str>) -> Result<BrowserBackendKind> {
    let normalized = normalize_backend_name(raw.unwrap_or("native_cdp"));
    match normalized.as_str() {
        "native" | "native_cdp" | "native_browser" | "rust" => Ok(BrowserBackendKind::NativeCdp),
        "agent_browser" | "agent_browser_cli" | "agentbrowser" => {
            Ok(BrowserBackendKind::AgentBrowserCli)
        }
        other => bail!(
            "unsupported browser backend '{}'; expected native_cdp or agent_browser_cli",
            other
        ),
    }
}

fn browser_sessions_root_for(workspace_root: &Path) -> PathBuf {
    browser_root_for(workspace_root).join("sessions")
}

fn browser_state_root_for(workspace_root: &Path) -> PathBuf {
    browser_root_for(workspace_root).join("state")
}

fn session_record_path(workspace_root: &Path, session_id: &str) -> PathBuf {
    browser_sessions_root_for(workspace_root).join(format!("{session_id}.json"))
}

fn workspace_relative_path(workspace_root: &Path, path: &Path) -> String {
    path.strip_prefix(workspace_root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn absolute_workspace_path(workspace_root: &Path, path: &str) -> PathBuf {
    let value = PathBuf::from(path);
    if value.is_absolute() {
        value
    } else {
        workspace_root.join(value)
    }
}

fn record_to_summary(record: BrowserSessionRecord) -> BrowserSessionSummary {
    BrowserSessionSummary {
        id: record.id,
        label: record.label,
        backend: record.backend,
        created_at: record.created_at,
        updated_at: record.updated_at,
        state_path: record.state_path,
    }
}

fn save_session_record(workspace_root: &Path, record: &BrowserSessionRecord) -> Result<()> {
    let path = session_record_path(workspace_root, &record.id);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create '{}'", parent.display()))?;
    }
    fs::write(&path, serde_json::to_vec_pretty(record)?)
        .with_context(|| format!("Failed to write '{}'", path.display()))?;
    Ok(())
}

fn load_session_record(workspace_root: &Path, session_id: &str) -> Result<BrowserSessionRecord> {
    let path = session_record_path(workspace_root, session_id);
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read '{}'", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("Failed to parse '{}'", path.display()))
}

fn load_session_for_request(
    workspace_root: &Path,
    session_id: Option<&str>,
) -> Result<Option<BrowserSessionRecord>> {
    match session_id {
        Some(session_id) => Ok(Some(load_session_record(workspace_root, session_id)?)),
        None => Ok(None),
    }
}

fn session_state_path(workspace_root: &Path, record: &BrowserSessionRecord) -> PathBuf {
    absolute_workspace_path(workspace_root, &record.state_path)
}

fn save_session_state(path: &Path, state: &BrowserSessionState) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create '{}'", parent.display()))?;
    }
    fs::write(path, serde_json::to_vec_pretty(state)?)
        .with_context(|| format!("Failed to write '{}'", path.display()))?;
    Ok(())
}

fn load_session_state(path: &Path) -> Result<BrowserSessionState> {
    if !path.exists() {
        return Ok(BrowserSessionState::default());
    }
    let raw =
        fs::read_to_string(path).with_context(|| format!("Failed to read '{}'", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("Failed to parse '{}'", path.display()))
}

fn load_session_state_for_record(
    workspace_root: &Path,
    record: Option<&BrowserSessionRecord>,
) -> Result<Option<BrowserSessionState>> {
    match record {
        Some(record) => Ok(Some(load_session_state(&session_state_path(
            workspace_root,
            record,
        ))?)),
        None => Ok(None),
    }
}

async fn persist_page_session_state(
    workspace_root: &Path,
    record: &BrowserSessionRecord,
    page: &Page,
    base_state: Option<&BrowserSessionState>,
) -> Result<()> {
    let mut next_state = base_state.cloned().unwrap_or_default();
    next_state.cookies = page
        .cookies()
        .await
        .unwrap_or_else(|_| next_state.cookies.clone());

    let local_keys: Vec<_> = next_state.local_storage.keys().cloned().collect();
    for key in local_keys {
        if let Some(value) = page.local_storage(&key).await? {
            next_state.local_storage.insert(key, value);
        }
    }
    let session_keys: Vec<_> = next_state.session_storage.keys().cloned().collect();
    for key in session_keys {
        if let Some(value) = page.session_storage(&key).await? {
            next_state.session_storage.insert(key, value);
        }
    }

    save_session_state(&session_state_path(workspace_root, record), &next_state)?;
    let mut updated = record.clone();
    updated.updated_at = chrono::Utc::now().to_rfc3339();
    save_session_record(workspace_root, &updated)?;
    Ok(())
}

async fn restore_page_session_state(page: &Page, state: &BrowserSessionState) -> Result<()> {
    for (key, value) in &state.local_storage {
        page.set_local_storage(key, value).await?;
    }
    for (key, value) in &state.session_storage {
        page.set_session_storage(key, value).await?;
    }
    Ok(())
}

async fn inspect_native(
    workspace_root: &Path,
    request: &BrowserInspectRequest,
    session: Option<&BrowserSessionRecord>,
) -> Result<BrowserInspectResult> {
    let browser = new_browser(workspace_root, request.timeout_ms).await?;
    let session_state = load_session_state_for_record(workspace_root, session)?;
    let page = open_page(
        &browser,
        &request.url,
        request.wait_until.as_deref(),
        session_state.as_ref(),
    )
    .await?;
    let title = page.title().await.unwrap_or_default();
    let url = page
        .url()
        .await
        .unwrap_or_else(|_| normalize_url(&request.url));
    let data = inspect_page(&page, request).await?;
    let artifact_path =
        resolve_output_path(workspace_root, request.path.as_deref(), "inspect", "json")?;
    fs::write(
        &artifact_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "backend": BrowserBackendKind::NativeCdp.as_str(),
            "session_id": request.session_id,
            "url": url,
            "title": title,
            "kind": request.kind,
            "data": data,
        }))?,
    )
    .with_context(|| format!("Failed to write '{}'", artifact_path.display()))?;
    if let Some(record) = session {
        persist_page_session_state(workspace_root, record, &page, session_state.as_ref()).await?;
    }
    browser.close().await?;
    Ok(BrowserInspectResult {
        backend: BrowserBackendKind::NativeCdp.as_str().to_string(),
        session_id: request.session_id.clone(),
        url,
        title,
        kind: request.kind.clone(),
        data,
        artifact_path: artifact_path.display().to_string(),
    })
}

async fn inspect_agent_browser(
    workspace_root: &Path,
    request: &BrowserInspectRequest,
    session: Option<&BrowserSessionRecord>,
) -> Result<BrowserInspectResult> {
    let normalized = normalize_url(&request.url);
    let open_result =
        agent_browser_open_url(workspace_root, session, request.timeout_ms, &normalized).await?;
    let data = agent_browser_inspect(workspace_root, session, request).await?;
    let title = agent_browser_get(workspace_root, session, request.timeout_ms, "title", None)
        .await?
        .as_str()
        .filter(|value| !value.is_empty())
        .or_else(|| {
            open_result
                .get("data")
                .and_then(|data| data.get("title"))
                .and_then(Value::as_str)
        })
        .unwrap_or_default()
        .to_string();
    let url = agent_browser_get(workspace_root, session, request.timeout_ms, "url", None)
        .await?
        .as_str()
        .filter(|value| !value.is_empty())
        .or_else(|| {
            open_result
                .get("data")
                .and_then(|data| data.get("url"))
                .and_then(Value::as_str)
        })
        .unwrap_or(&normalized)
        .to_string();
    let artifact_path =
        resolve_output_path(workspace_root, request.path.as_deref(), "inspect", "json")?;
    fs::write(
        &artifact_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "backend": BrowserBackendKind::AgentBrowserCli.as_str(),
            "session_id": request.session_id,
            "url": url,
            "title": title,
            "kind": request.kind,
            "data": data,
        }))?,
    )
    .with_context(|| format!("Failed to write '{}'", artifact_path.display()))?;
    Ok(BrowserInspectResult {
        backend: BrowserBackendKind::AgentBrowserCli.as_str().to_string(),
        session_id: request.session_id.clone(),
        url,
        title,
        kind: request.kind.clone(),
        data,
        artifact_path: artifact_path.display().to_string(),
    })
}

async fn run_native_sequence(
    workspace_root: &Path,
    request: &BrowserRunSequenceRequest,
    session: Option<&BrowserSessionRecord>,
) -> Result<BrowserRunSequenceResult> {
    let browser = new_browser(workspace_root, request.timeout_ms).await?;
    let mut session_state =
        load_session_state_for_record(workspace_root, session)?.unwrap_or_default();
    let page = browser.new_page().await?;
    if !session_state.cookies.is_empty() {
        page.add_cookies(session_state.cookies.clone()).await?;
    }

    let mut results = Vec::new();
    let mut session_applied = false;
    for (index, step) in request.steps.iter().enumerate() {
        let action = step.action.trim().to_ascii_lowercase();
        let detail = match action.as_str() {
            "navigate" => {
                let url = step.url.as_deref().context("navigate step requires url")?;
                page.goto(&normalize_url(url)).await?;
                if !session_applied {
                    restore_page_session_state(&page, &session_state).await?;
                    if !session_state.local_storage.is_empty()
                        || !session_state.session_storage.is_empty()
                    {
                        page.reload().await?;
                    }
                    session_applied = true;
                }
                page.wait_for_load_state(parse_load_state(step.wait_until.as_deref()))
                    .await?;
                serde_json::json!({
                    "url": page.url().await.unwrap_or_else(|_| normalize_url(url)),
                    "title": page.title().await.unwrap_or_default(),
                })
            }
            "wait" => {
                if let Some(selector) = step.selector.as_deref() {
                    page.wait_for_selector(selector).await?;
                    serde_json::json!({"selector": selector, "ready": true})
                } else if let Some(wait_ms) = step.wait_ms {
                    page.wait_for_timeout(wait_ms).await?;
                    serde_json::json!({"wait_ms": wait_ms})
                } else {
                    bail!("wait step requires selector or wait_ms");
                }
            }
            "click" => {
                let selector = step
                    .selector
                    .as_deref()
                    .context("click step requires selector")?;
                page.click(selector).await?;
                serde_json::json!({"selector": selector})
            }
            "type" => {
                let selector = step
                    .selector
                    .as_deref()
                    .context("type step requires selector")?;
                let text = step.text.as_deref().context("type step requires text")?;
                page.type_text(selector, text).await?;
                serde_json::json!({"selector": selector, "text_len": text.len()})
            }
            "fill" => {
                let selector = step
                    .selector
                    .as_deref()
                    .context("fill step requires selector")?;
                let text = step.text.as_deref().context("fill step requires text")?;
                page.fill(selector, text).await?;
                serde_json::json!({"selector": selector, "text_len": text.len()})
            }
            "press" => {
                let key = step.key.as_deref().context("press step requires key")?;
                page.press(key).await?;
                serde_json::json!({"key": key})
            }
            "select" => {
                let selector = step
                    .selector
                    .as_deref()
                    .context("select step requires selector")?;
                let values = step
                    .values
                    .clone()
                    .filter(|values| !values.is_empty())
                    .context("select step requires values")?;
                let element = page
                    .query_selector(selector)
                    .await?
                    .with_context(|| format!("Element not found for selector '{}'", selector))?;
                element.select_option(values.clone()).await?;
                serde_json::json!({"selector": selector, "values": values})
            }
            "extract" => {
                let data = extract_from_page(
                    &page,
                    &BrowserExtractRequest {
                        url: page.url().await.unwrap_or_default(),
                        backend: BrowserBackendKind::NativeCdp.as_str().to_string(),
                        session_id: None,
                        what: step.what.clone().unwrap_or_else(default_extract_kind),
                        selector: step.selector.clone(),
                        wait_until: None,
                        timeout_ms: request.timeout_ms,
                        max_results: None,
                        max_chars: step.max_chars,
                    },
                )
                .await?;
                serde_json::json!({ "data": data })
            }
            "screenshot" => {
                let result = screenshot_current_page(workspace_root, &page, step).await?;
                serde_json::to_value(result)?
            }
            "pdf" => {
                let result = pdf_current_page(workspace_root, &page, step).await?;
                serde_json::to_value(result)?
            }
            "set_local_storage" => {
                let key = step
                    .storage_key
                    .as_deref()
                    .context("set_local_storage step requires storage_key")?;
                let value = step
                    .storage_value
                    .as_deref()
                    .context("set_local_storage step requires storage_value")?;
                page.set_local_storage(key, value).await?;
                session_state
                    .local_storage
                    .insert(key.to_string(), value.to_string());
                serde_json::json!({"key": key})
            }
            "set_session_storage" => {
                let key = step
                    .storage_key
                    .as_deref()
                    .context("set_session_storage step requires storage_key")?;
                let value = step
                    .storage_value
                    .as_deref()
                    .context("set_session_storage step requires storage_value")?;
                page.set_session_storage(key, value).await?;
                session_state
                    .session_storage
                    .insert(key.to_string(), value.to_string());
                serde_json::json!({"key": key})
            }
            "reload" => {
                page.reload().await?;
                serde_json::json!({"reloaded": true})
            }
            "back" => {
                page.go_back().await?;
                serde_json::json!({"direction": "back"})
            }
            "forward" => {
                page.go_forward().await?;
                serde_json::json!({"direction": "forward"})
            }
            other => bail!("unsupported browser step action '{}'", other),
        };
        results.push(BrowserActionStepResult {
            index,
            action,
            ok: true,
            detail,
        });
    }

    let final_url = page.url().await.unwrap_or_default();
    let title = page.title().await.unwrap_or_default();
    if let Some(record) = session {
        persist_page_session_state(workspace_root, record, &page, Some(&session_state)).await?;
    }
    browser.close().await?;
    let artifact_path =
        resolve_output_path(workspace_root, request.path.as_deref(), "sequences", "json")?;
    fs::write(
        &artifact_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "backend": BrowserBackendKind::NativeCdp.as_str(),
            "session_id": request.session_id,
            "final_url": final_url,
            "title": title,
            "steps": results,
        }))?,
    )
    .with_context(|| format!("Failed to write '{}'", artifact_path.display()))?;
    Ok(BrowserRunSequenceResult {
        backend: BrowserBackendKind::NativeCdp.as_str().to_string(),
        session_id: request.session_id.clone(),
        final_url,
        title,
        steps: results,
        artifact_path: artifact_path.display().to_string(),
    })
}

async fn run_agent_browser_sequence(
    workspace_root: &Path,
    request: &BrowserRunSequenceRequest,
    session: Option<&BrowserSessionRecord>,
) -> Result<BrowserRunSequenceResult> {
    let mut results = Vec::new();
    let mut last_url = String::new();
    let mut last_title = String::new();
    for (index, step) in request.steps.iter().enumerate() {
        let action = step.action.trim().to_ascii_lowercase();
        let detail = match action.as_str() {
            "navigate" => {
                let url = step.url.as_deref().context("navigate step requires url")?;
                let response = agent_browser_open_url(
                    workspace_root,
                    session,
                    request.timeout_ms,
                    &normalize_url(url),
                )
                .await?;
                if let Some(value) = response
                    .get("data")
                    .and_then(|data| data.get("url"))
                    .and_then(Value::as_str)
                {
                    last_url = value.to_string();
                }
                if let Some(value) = response
                    .get("data")
                    .and_then(|data| data.get("title"))
                    .and_then(Value::as_str)
                {
                    last_title = value.to_string();
                }
                response
            }
            "wait" => {
                let wait_target = step
                    .selector
                    .clone()
                    .or_else(|| step.wait_ms.map(|value| value.to_string()))
                    .context("wait step requires selector or wait_ms")?;
                run_agent_browser_command(
                    workspace_root,
                    session,
                    request.timeout_ms,
                    &["wait".to_string(), wait_target],
                )
                .await?
            }
            "click" => {
                let selector = step
                    .selector
                    .as_deref()
                    .context("click step requires selector")?;
                run_agent_browser_command(
                    workspace_root,
                    session,
                    request.timeout_ms,
                    &["click".to_string(), selector.to_string()],
                )
                .await?
            }
            "type" => {
                let selector = step
                    .selector
                    .as_deref()
                    .context("type step requires selector")?;
                let text = step.text.as_deref().context("type step requires text")?;
                run_agent_browser_command(
                    workspace_root,
                    session,
                    request.timeout_ms,
                    &["type".to_string(), selector.to_string(), text.to_string()],
                )
                .await?
            }
            "fill" => {
                let selector = step
                    .selector
                    .as_deref()
                    .context("fill step requires selector")?;
                let text = step.text.as_deref().context("fill step requires text")?;
                run_agent_browser_command(
                    workspace_root,
                    session,
                    request.timeout_ms,
                    &["fill".to_string(), selector.to_string(), text.to_string()],
                )
                .await?
            }
            "press" => {
                let key = step.key.as_deref().context("press step requires key")?;
                run_agent_browser_command(
                    workspace_root,
                    session,
                    request.timeout_ms,
                    &["press".to_string(), key.to_string()],
                )
                .await?
            }
            "select" => {
                let selector = step
                    .selector
                    .as_deref()
                    .context("select step requires selector")?;
                let values = step
                    .values
                    .clone()
                    .filter(|values| !values.is_empty())
                    .context("select step requires values")?;
                let mut args = vec!["select".to_string(), selector.to_string()];
                args.extend(values);
                run_agent_browser_command(workspace_root, session, request.timeout_ms, &args)
                    .await?
            }
            "extract" => {
                let extract_request = BrowserExtractRequest {
                    url: agent_browser_get(
                        workspace_root,
                        session,
                        request.timeout_ms,
                        "url",
                        None,
                    )
                    .await?
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                    backend: BrowserBackendKind::AgentBrowserCli.as_str().to_string(),
                    session_id: request.session_id.clone(),
                    what: step.what.clone().unwrap_or_else(default_extract_kind),
                    selector: step.selector.clone(),
                    wait_until: None,
                    timeout_ms: request.timeout_ms,
                    max_results: None,
                    max_chars: step.max_chars,
                };
                serde_json::json!({
                    "data": agent_browser_extract(workspace_root, session, request.timeout_ms, &extract_request).await?
                })
            }
            "screenshot" => {
                let path = resolve_output_path(
                    workspace_root,
                    step.path.as_deref(),
                    "screenshots",
                    step.format
                        .as_deref()
                        .map(|value| {
                            if value.eq_ignore_ascii_case("jpeg")
                                || value.eq_ignore_ascii_case("jpg")
                            {
                                "jpg"
                            } else {
                                "png"
                            }
                        })
                        .unwrap_or("png"),
                )?;
                let mut args = vec!["screenshot".to_string(), path.display().to_string()];
                if step.full_page.unwrap_or(false) {
                    args.push("--full".to_string());
                }
                run_agent_browser_command(workspace_root, session, request.timeout_ms, &args)
                    .await?;
                serde_json::json!({"path": path.display().to_string()})
            }
            "pdf" => {
                let path = resolve_output_path(workspace_root, step.path.as_deref(), "pdf", "pdf")?;
                run_agent_browser_command(
                    workspace_root,
                    session,
                    request.timeout_ms,
                    &["pdf".to_string(), path.display().to_string()],
                )
                .await?;
                serde_json::json!({
                    "path": path.display().to_string(),
                    "bytes": fs::metadata(&path).map(|metadata| metadata.len()).unwrap_or(0),
                })
            }
            "set_local_storage" => {
                let key = step
                    .storage_key
                    .as_deref()
                    .context("set_local_storage step requires storage_key")?;
                let value = step
                    .storage_value
                    .as_deref()
                    .context("set_local_storage step requires storage_value")?;
                agent_browser_eval_json(
                    workspace_root,
                    session,
                    request.timeout_ms,
                    &format!(
                        "(() => {{ localStorage.setItem({key:?}, {value:?}); return {{ key: {key:?} }}; }})()"
                    ),
                )
                .await?
            }
            "set_session_storage" => {
                let key = step
                    .storage_key
                    .as_deref()
                    .context("set_session_storage step requires storage_key")?;
                let value = step
                    .storage_value
                    .as_deref()
                    .context("set_session_storage step requires storage_value")?;
                agent_browser_eval_json(
                    workspace_root,
                    session,
                    request.timeout_ms,
                    &format!(
                        "(() => {{ sessionStorage.setItem({key:?}, {value:?}); return {{ key: {key:?} }}; }})()"
                    ),
                )
                .await?
            }
            "reload" => {
                run_agent_browser_command(
                    workspace_root,
                    session,
                    request.timeout_ms,
                    &["reload".to_string()],
                )
                .await?
            }
            "back" => {
                run_agent_browser_command(
                    workspace_root,
                    session,
                    request.timeout_ms,
                    &["back".to_string()],
                )
                .await?
            }
            "forward" => {
                run_agent_browser_command(
                    workspace_root,
                    session,
                    request.timeout_ms,
                    &["forward".to_string()],
                )
                .await?
            }
            other => bail!("unsupported browser step action '{}'", other),
        };
        results.push(BrowserActionStepResult {
            index,
            action,
            ok: true,
            detail: detail.get("data").cloned().unwrap_or(detail),
        });
    }

    let final_url = agent_browser_get(workspace_root, session, request.timeout_ms, "url", None)
        .await?
        .as_str()
        .filter(|value| !value.is_empty())
        .unwrap_or(&last_url)
        .to_string();
    let title = agent_browser_get(workspace_root, session, request.timeout_ms, "title", None)
        .await?
        .as_str()
        .filter(|value| !value.is_empty())
        .unwrap_or(&last_title)
        .to_string();
    let final_url = if final_url.is_empty() {
        results
            .iter()
            .rev()
            .find_map(|step| {
                step.detail
                    .get("url")
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
            })
            .unwrap_or(final_url)
    } else {
        final_url
    };
    let title = if title.is_empty() {
        results
            .iter()
            .rev()
            .find_map(|step| {
                step.detail
                    .get("title")
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
            })
            .unwrap_or(title)
    } else {
        title
    };
    let artifact_path =
        resolve_output_path(workspace_root, request.path.as_deref(), "sequences", "json")?;
    fs::write(
        &artifact_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "backend": BrowserBackendKind::AgentBrowserCli.as_str(),
            "session_id": request.session_id,
            "final_url": final_url,
            "title": title,
            "steps": results,
        }))?,
    )
    .with_context(|| format!("Failed to write '{}'", artifact_path.display()))?;
    Ok(BrowserRunSequenceResult {
        backend: BrowserBackendKind::AgentBrowserCli.as_str().to_string(),
        session_id: request.session_id.clone(),
        final_url,
        title,
        steps: results,
        artifact_path: artifact_path.display().to_string(),
    })
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
    session_state: Option<&BrowserSessionState>,
) -> Result<openrustclaw_automation::Page> {
    let page = browser.new_page().await?;
    if let Some(session_state) = session_state {
        if !session_state.cookies.is_empty() {
            page.add_cookies(session_state.cookies.clone()).await?;
        }
    }
    page.goto(&normalize_url(url)).await?;
    if let Some(session_state) = session_state {
        restore_page_session_state(&page, session_state).await?;
        if !session_state.local_storage.is_empty() || !session_state.session_storage.is_empty() {
            page.reload().await?;
        }
    }
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

async fn extract_from_page(page: &Page, request: &BrowserExtractRequest) -> Result<Value> {
    let what = request.what.trim().to_lowercase();
    let max_results = request.max_results.unwrap_or(100);
    let max_chars = request.max_chars.unwrap_or(4000);
    let data = match what.as_str() {
        "html" => Value::String(truncate_string(page.content().await?, max_chars)),
        "links" => limit_value_array(page.evaluate(links_script()).await?, max_results),
        "images" => limit_value_array(page.evaluate(images_script()).await?, max_results),
        "headings" => limit_value_array(page.evaluate(headings_script()).await?, max_results),
        "forms" => limit_value_array(page.evaluate(forms_script()).await?, max_results),
        "selector" => {
            let selector = request
                .selector
                .as_deref()
                .context("selector is required when what=selector")?;
            limit_value_array(
                page.evaluate(&selector_extract_script(selector)).await?,
                max_results,
            )
        }
        _ => {
            let value = page.evaluate(text_extract_script()).await?;
            Value::String(truncate_string(
                value.as_str().unwrap_or_default().to_string(),
                max_chars,
            ))
        }
    };
    Ok(data)
}

async fn inspect_page(page: &Page, request: &BrowserInspectRequest) -> Result<Value> {
    let kind = request.kind.trim().to_ascii_lowercase();
    match kind.as_str() {
        "links" => Ok(page.evaluate(links_script()).await?),
        "images" => Ok(page.evaluate(images_script()).await?),
        "headings" => Ok(page.evaluate(headings_script()).await?),
        "forms" => Ok(page.evaluate(forms_script()).await?),
        "html" => Ok(Value::String(page.content().await?)),
        _ => Ok(page
            .evaluate(&snapshot_script(
                request.selector.as_deref(),
                request.snapshot_depth.unwrap_or(5),
                request.interactive_only,
            ))
            .await?),
    }
}

fn links_script() -> &'static str {
    r#"
    Array.from(document.querySelectorAll('a[href]')).map(a => ({
        text: (a.textContent || '').trim(),
        href: a.href,
        title: a.title || ''
    }))
    "#
}

fn images_script() -> &'static str {
    r#"
    Array.from(document.querySelectorAll('img')).map(img => ({
        src: img.src,
        alt: img.alt || '',
        width: img.naturalWidth || null,
        height: img.naturalHeight || null
    })).filter(img => img.src)
    "#
}

fn headings_script() -> &'static str {
    r#"
    Array.from(document.querySelectorAll('h1, h2, h3, h4, h5, h6')).map(node => ({
        level: Number(node.tagName.substring(1)),
        text: (node.textContent || '').trim()
    }))
    "#
}

fn forms_script() -> &'static str {
    r#"
    Array.from(document.querySelectorAll('form')).map((form, index) => ({
        index,
        id: form.id || null,
        action: form.action || null,
        method: form.method || null,
        inputs: Array.from(form.querySelectorAll('input, select, textarea, button')).map(input => ({
            tag: input.tagName.toLowerCase(),
            type: input.getAttribute('type') || null,
            name: input.getAttribute('name') || null,
            id: input.id || null,
            label: input.getAttribute('aria-label') || null,
            placeholder: input.getAttribute('placeholder') || null
        }))
    }))
    "#
}

fn selector_extract_script(selector: &str) -> String {
    format!(
        r#"
        Array.from(document.querySelectorAll({selector:?})).map(node => ({{
            tag: node.tagName.toLowerCase(),
            text: (node.textContent || '').trim(),
            html: node.innerHTML || ''
        }}))
        "#
    )
}

fn text_extract_script() -> &'static str {
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
    "#
}

fn snapshot_script(selector: Option<&str>, depth: usize, interactive_only: bool) -> String {
    let selector_literal = selector
        .map(|value| format!("{value:?}"))
        .unwrap_or_else(|| "null".to_string());
    format!(
        r#"
        (function() {{
            const scopeSelector = {selector_literal};
            const interactiveOnly = {interactive_only};
            const maxDepth = {depth};
            const root = scopeSelector ? document.querySelector(scopeSelector) : document.body;
            if (!root) {{
                return {{ error: `selector not found: ${{scopeSelector}}` }};
            }}
            function summarize(node, level) {{
                if (!node || node.nodeType !== Node.ELEMENT_NODE || level > maxDepth) {{
                    return null;
                }}
                const el = node;
                const role = el.getAttribute('role') || null;
                const ariaLabel = el.getAttribute('aria-label') || null;
                const interactive = !!(
                    el.matches('a,button,input,select,textarea,[tabindex],[onclick],[contenteditable=\"true\"]')
                    || role
                );
                const children = Array.from(el.children)
                    .map(child => summarize(child, level + 1))
                    .filter(Boolean);
                if (interactiveOnly && !interactive && !children.length && level > 0) {{
                    return null;
                }}
                const summary = {{
                    tag: el.tagName.toLowerCase(),
                    role,
                    aria_label: ariaLabel,
                    id: el.id || null,
                    classes: typeof el.className === 'string' ? el.className : null,
                    text: (el.innerText || el.textContent || '').trim().replace(/\s+/g, ' ').slice(0, 180) || null,
                    href: el.href || null,
                    name: el.getAttribute('name') || null,
                    interactive
                }};
                if (children.length) {{
                    summary.children = children;
                }}
                return summary;
            }}
            return summarize(root, 0);
        }})()
        "#,
        selector_literal = selector_literal,
        interactive_only = if interactive_only { "true" } else { "false" },
        depth = depth
    )
}

async fn screenshot_current_page(
    workspace_root: &Path,
    page: &Page,
    step: &BrowserActionStep,
) -> Result<BrowserScreenshotResult> {
    let screenshot = if let Some(selector) = step.selector.as_deref() {
        let element = page
            .query_selector(selector)
            .await?
            .with_context(|| format!("Element not found for selector '{}'", selector))?;
        element.screenshot().await?
    } else {
        page.screenshot_with_options(ScreenshotOptions {
            format: parse_screenshot_format(step.format.as_deref()),
            quality: None,
            clip: None,
            full_page: step.full_page.unwrap_or(false),
            hide_selectors: vec![],
        })
        .await?
    };
    let resolved_path = resolve_output_path(
        workspace_root,
        step.path.as_deref(),
        "screenshots",
        match screenshot.format {
            ScreenshotFormat::Png => "png",
            ScreenshotFormat::Jpeg => "jpg",
        },
    )?;
    screenshot.save(&resolved_path.to_string_lossy())?;
    Ok(BrowserScreenshotResult {
        backend: BrowserBackendKind::NativeCdp.as_str().to_string(),
        url: page.url().await.unwrap_or_default(),
        title: page.title().await.unwrap_or_default(),
        path: resolved_path.display().to_string(),
        width: screenshot.width,
        height: screenshot.height,
        format: match screenshot.format {
            ScreenshotFormat::Png => "png".to_string(),
            ScreenshotFormat::Jpeg => "jpeg".to_string(),
        },
    })
}

async fn pdf_current_page(
    workspace_root: &Path,
    page: &Page,
    step: &BrowserActionStep,
) -> Result<BrowserPdfResult> {
    let path = resolve_output_path(workspace_root, step.path.as_deref(), "pdf", "pdf")?;
    let pdf = page
        .pdf(PdfOptions {
            format: step.format.clone().or_else(|| Some("A4".to_string())),
            print_background: true,
            ..Default::default()
        })
        .await?;
    fs::write(&path, &pdf).with_context(|| format!("Failed to write '{}'", path.display()))?;
    Ok(BrowserPdfResult {
        backend: BrowserBackendKind::NativeCdp.as_str().to_string(),
        url: page.url().await.unwrap_or_default(),
        title: page.title().await.unwrap_or_default(),
        path: path.display().to_string(),
        bytes: pdf.len(),
    })
}

fn agent_browser_binary() -> String {
    std::env::var("AGENT_BROWSER_BIN").unwrap_or_else(|_| "agent-browser".to_string())
}

fn configure_isolated_command_env(command: &mut Command, policy: &ExternalBackendPolicy) {
    command.env_clear();
    for key in &policy.command_env_allowlist {
        if let Ok(value) = std::env::var(key) {
            command.env(key, value);
        }
    }
}

async fn run_agent_browser_command(
    workspace_root: &Path,
    session: Option<&BrowserSessionRecord>,
    timeout_ms: Option<u64>,
    args: &[String],
) -> Result<Value> {
    let policy = backend_policy(workspace_root)?;
    ensure_backend_execution_allowed_with_policy(
        workspace_root,
        &policy,
        BrowserBackendKind::AgentBrowserCli,
        args.first().map(String::as_str).unwrap_or("agent_browser"),
        session.map(|value| value.id.as_str()),
    )?;
    let mut command = Command::new(agent_browser_binary());
    configure_isolated_command_env(&mut command, &policy);
    command.arg("--json");
    command.arg("--download-path").arg(
        browser_root_for(workspace_root)
            .join("downloads")
            .display()
            .to_string(),
    );
    if let Some(session) = session {
        command.arg("--session").arg(&session.id);
    }
    if args.iter().any(|arg| arg.starts_with("file://")) {
        command.arg("--allow-file-access");
    }
    for arg in args {
        command.arg(arg);
    }

    let output = timeout(
        Duration::from_millis(timeout_ms.unwrap_or(25_000).max(1)),
        command.output(),
    )
    .await
    .context("agent-browser command timed out")??;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let parsed = if stdout.is_empty() {
        serde_json::json!({
            "success": output.status.success(),
            "stderr": stderr,
        })
    } else {
        serde_json::from_str(&stdout).unwrap_or_else(|_| {
            serde_json::json!({
                "success": output.status.success(),
                "stdout": stdout,
                "stderr": stderr,
            })
        })
    };
    if !output.status.success() {
        let message = parsed
            .get("error")
            .and_then(Value::as_str)
            .map(ToString::to_string)
            .filter(|value| !value.is_empty())
            .or_else(|| (!stderr.is_empty()).then_some(stderr.clone()))
            .unwrap_or_else(|| "agent-browser command failed".to_string());
        record_external_backend_audit(
            workspace_root,
            &policy,
            BrowserBackendKind::AgentBrowserCli,
            args.first().map(String::as_str).unwrap_or("agent_browser"),
            session.map(|value| value.id.as_str()),
            true,
            false,
            Some(message.clone()),
        )?;
        bail!("{message}");
    }
    record_external_backend_audit(
        workspace_root,
        &policy,
        BrowserBackendKind::AgentBrowserCli,
        args.first().map(String::as_str).unwrap_or("agent_browser"),
        session.map(|value| value.id.as_str()),
        true,
        true,
        None,
    )?;
    Ok(parsed)
}

async fn agent_browser_open_url(
    workspace_root: &Path,
    session: Option<&BrowserSessionRecord>,
    timeout_ms: Option<u64>,
    url: &str,
) -> Result<Value> {
    let _ = run_agent_browser_command(workspace_root, session, timeout_ms, &["close".to_string()])
        .await;
    run_agent_browser_command(
        workspace_root,
        session,
        timeout_ms,
        &["open".to_string(), url.to_string()],
    )
    .await
}

async fn agent_browser_get(
    workspace_root: &Path,
    session: Option<&BrowserSessionRecord>,
    timeout_ms: Option<u64>,
    what: &str,
    selector: Option<&str>,
) -> Result<Value> {
    let mut args = vec!["get".to_string(), what.to_string()];
    if let Some(selector) = selector {
        args.push(selector.to_string());
    }
    let output = run_agent_browser_command(workspace_root, session, timeout_ms, &args).await?;
    Ok(output.get("data").cloned().unwrap_or(Value::Null))
}

async fn agent_browser_eval_json(
    workspace_root: &Path,
    session: Option<&BrowserSessionRecord>,
    timeout_ms: Option<u64>,
    script: &str,
) -> Result<Value> {
    let output = run_agent_browser_command(
        workspace_root,
        session,
        timeout_ms,
        &["eval".to_string(), script.to_string()],
    )
    .await?;
    let data = output.get("data").cloned().unwrap_or(Value::Null);
    if let Some(raw) = data.as_str() {
        serde_json::from_str(raw).or(Ok(Value::String(raw.to_string())))
    } else {
        Ok(data)
    }
}

async fn agent_browser_extract(
    workspace_root: &Path,
    session: Option<&BrowserSessionRecord>,
    timeout_ms: Option<u64>,
    request: &BrowserExtractRequest,
) -> Result<Value> {
    let what = request.what.trim().to_ascii_lowercase();
    let max_chars = request.max_chars.unwrap_or(4000);
    let max_results = request.max_results.unwrap_or(100);
    match what.as_str() {
        "html" => {
            let html = agent_browser_get(
                workspace_root,
                session,
                timeout_ms,
                "html",
                request.selector.as_deref(),
            )
            .await?;
            Ok(Value::String(truncate_string(
                html.as_str().unwrap_or_default().to_string(),
                max_chars,
            )))
        }
        "links" => Ok(limit_value_array(
            agent_browser_eval_json(workspace_root, session, timeout_ms, links_script()).await?,
            max_results,
        )),
        "images" => Ok(limit_value_array(
            agent_browser_eval_json(workspace_root, session, timeout_ms, images_script()).await?,
            max_results,
        )),
        "headings" => Ok(limit_value_array(
            agent_browser_eval_json(workspace_root, session, timeout_ms, headings_script()).await?,
            max_results,
        )),
        "forms" => Ok(limit_value_array(
            agent_browser_eval_json(workspace_root, session, timeout_ms, forms_script()).await?,
            max_results,
        )),
        "selector" => {
            let selector = request
                .selector
                .as_deref()
                .context("selector is required when what=selector")?;
            Ok(limit_value_array(
                agent_browser_eval_json(
                    workspace_root,
                    session,
                    timeout_ms,
                    &selector_extract_script(selector),
                )
                .await?,
                max_results,
            ))
        }
        _ => {
            let value =
                agent_browser_eval_json(workspace_root, session, timeout_ms, text_extract_script())
                    .await?;
            Ok(Value::String(truncate_string(
                value.as_str().unwrap_or_default().to_string(),
                max_chars,
            )))
        }
    }
}

async fn agent_browser_inspect(
    workspace_root: &Path,
    session: Option<&BrowserSessionRecord>,
    request: &BrowserInspectRequest,
) -> Result<Value> {
    let kind = request.kind.trim().to_ascii_lowercase();
    match kind.as_str() {
        "links" => {
            agent_browser_eval_json(workspace_root, session, request.timeout_ms, links_script())
                .await
        }
        "images" => {
            agent_browser_eval_json(workspace_root, session, request.timeout_ms, images_script())
                .await
        }
        "headings" => {
            agent_browser_eval_json(
                workspace_root,
                session,
                request.timeout_ms,
                headings_script(),
            )
            .await
        }
        "forms" => {
            agent_browser_eval_json(workspace_root, session, request.timeout_ms, forms_script())
                .await
        }
        "html" => {
            agent_browser_get(
                workspace_root,
                session,
                request.timeout_ms,
                "html",
                request.selector.as_deref(),
            )
            .await
        }
        _ => {
            let mut args = vec!["snapshot".to_string()];
            if request.interactive_only {
                args.push("-i".to_string());
            }
            if let Some(depth) = request.snapshot_depth {
                args.push("-d".to_string());
                args.push(depth.to_string());
            }
            if let Some(selector) = request.selector.as_deref() {
                args.push("-s".to_string());
                args.push(selector.to_string());
            }
            let output =
                run_agent_browser_command(workspace_root, session, request.timeout_ms, &args)
                    .await?;
            Ok(output.get("data").cloned().unwrap_or(output))
        }
    }
}

fn normalize_url(url: &str) -> String {
    if url.starts_with("http://")
        || url.starts_with("https://")
        || url.starts_with("file://")
        || url.starts_with("data:")
    {
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
        BrowserBackendKind, ExternalBackendAuditEntry, append_external_backend_audit_entry,
        backend_policy, ensure_backend_execution_allowed_with_policy, extract_links, extract_title,
        html_to_text, list_backend_audit, normalize_url, parse_load_state, policy_from_config,
        resolve_output_path,
    };
    use openrustclaw_automation::browser::LoadState;
    use openrustclaw_core::config::AppConfig;
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

    #[test]
    fn backend_policy_defaults_include_agent_browser_cli() {
        let root = tempdir().unwrap();
        let policy = policy_from_config(&AppConfig::default(), root.path());
        assert!(
            policy
                .allowed_backends
                .contains(&"agent_browser_cli".to_string())
        );
        assert!(policy.allow_local_cli_wrappers);
    }

    #[test]
    fn denied_external_backend_execution_writes_audit_entry() {
        let root = tempdir().unwrap();
        let mut config = AppConfig::default();
        config.external_backends.allowed_backends = Vec::new();
        config.external_backends.allow_local_cli_wrappers = false;
        let policy = policy_from_config(&config, root.path());
        let error = ensure_backend_execution_allowed_with_policy(
            root.path(),
            &policy,
            BrowserBackendKind::AgentBrowserCli,
            "inspect",
            Some("session-1"),
        )
        .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("agent-browser backend is disabled by external backend policy")
        );
        let audit = list_backend_audit(root.path(), 10).unwrap();
        assert_eq!(audit.len(), 1);
        assert_eq!(audit[0].backend, "agent_browser_cli");
        assert!(!audit[0].allowed);
        assert!(!audit[0].success);
        assert_eq!(audit[0].action, "inspect");
    }

    #[test]
    fn backend_audit_entries_are_returned_newest_first() {
        let root = tempdir().unwrap();
        let policy = backend_policy(root.path()).unwrap();
        append_external_backend_audit_entry(
            root.path(),
            &policy,
            &ExternalBackendAuditEntry {
                timestamp: "2026-03-24T00:00:00Z".to_string(),
                backend: "agent_browser_cli".to_string(),
                transport: "local_cli_wrapper".to_string(),
                action: "inspect".to_string(),
                session_id: None,
                allowed: true,
                success: true,
                detail: None,
            },
        )
        .unwrap();
        append_external_backend_audit_entry(
            root.path(),
            &policy,
            &ExternalBackendAuditEntry {
                timestamp: "2026-03-25T00:00:00Z".to_string(),
                backend: "agent_browser_cli".to_string(),
                transport: "local_cli_wrapper".to_string(),
                action: "pdf".to_string(),
                session_id: None,
                allowed: true,
                success: false,
                detail: Some("boom".to_string()),
            },
        )
        .unwrap();
        let audit = list_backend_audit(root.path(), 10).unwrap();
        assert_eq!(audit.len(), 2);
        assert_eq!(audit[0].action, "pdf");
        assert_eq!(audit[1].action, "inspect");
    }
}
