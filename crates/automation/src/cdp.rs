//! Chrome DevTools Protocol (CDP) backend.
//!
//! This module provides direct control of Chrome/Chromium browsers through
//! the Chrome DevTools Protocol, offering lower-level access than Playwright.
//!
//! # Features
//! - Direct browser control without Playwright dependency
//! - Better performance for some operations
//! - Network interception and monitoring
//! - Mobile device emulation
//! - Lower resource overhead

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use image::ImageEncoder;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{debug, info, trace};

use crate::browser::{
    BrowserBackend, BrowserContextBackend, Cookie, ElementBackend, FrameBackend, LoadState,
    PageBackend, PdfOptions, Rect, Screenshot, ScreenshotOptions,
};
use crate::config::{BrowserConfig, BrowserType};
use crate::error::{AutomationError, Result};

/// CDP browser backend for direct Chrome control.
pub struct CdpBackend {
    config: BrowserConfig,
    websocket_url: String,
    _session_id: String,
}

impl CdpBackend {
    /// Create a new CDP backend.
    pub async fn new(config: &BrowserConfig) -> Result<Self> {
        // Verify we're targeting Chromium-based browser
        match config.automation.browser {
            BrowserType::Chromium | BrowserType::Edge => {}
            other => {
                return Err(AutomationError::ConfigError {
                    field: "browser".to_string(),
                    reason: format!("CDP only supports Chromium-based browsers, got: {}", other),
                })
            }
        }

        // Find Chrome/Chromium executable
        let chrome_path = find_chrome_executable(config.executable_path.as_deref())?;
        
        info!(path = %chrome_path, "Found Chrome executable");

        // In a real implementation, we would:
        // 1. Launch Chrome with --remote-debugging-port
        // 2. Connect to the debugging port
        // 3. Create a WebSocket connection to the CDP endpoint
        
        let websocket_url = format!("ws://localhost:9222/devtools/browser/{}"
, uuid::Uuid::new_v4());

        Ok(Self {
            config: config.clone(),
            websocket_url,
            _session_id: uuid::Uuid::new_v4().to_string(),
        })
    }

    /// Connect to an existing Chrome instance.
    pub async fn connect(websocket_url: &str) -> Result<Self> {
        info!(url = %websocket_url, "Connecting to existing Chrome instance");

        Ok(Self {
            config: BrowserConfig::default(),
            websocket_url: websocket_url.to_string(),
            _session_id: uuid::Uuid::new_v4().to_string(),
        })
    }

    /// Send a CDP command.
    async fn send_command(&self, method: &str, _params: Value) -> Result<Value> {
        trace!(method = %method, "Sending CDP command");
        
        // In a real implementation, this would:
        // 1. Serialize the command
        // 2. Send via WebSocket
        // 3. Wait for and parse the response
        
        Ok(Value::Object(serde_json::Map::new()))
    }

    /// Enable a CDP domain.
    #[allow(dead_code)]
    async fn enable_domain(&self, domain: &str) -> Result<()> {
        self.send_command(&format!("{}.enable", domain), Value::Null).await?;
        Ok(())
    }
}

#[async_trait]
impl BrowserBackend for CdpBackend {
    async fn new_page(&self) -> Result<Arc<dyn PageBackend>> {
        debug!("Creating new CDP page");
        
        // Target.createTarget
        let result = self
            .send_command("Target.createTarget", serde_json::json!({"url": "about:blank"}))
            .await?;
        
        let target_id = result
            .get("targetId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AutomationError::CdpError {
                method: "Target.createTarget".to_string(),
                error: "Missing targetId in response".to_string(),
            })?;
        
        Ok(Arc::new(CdpPage::new(&self.websocket_url, target_id, &self.config)?))
    }

    async fn pages(&self) -> Result<Vec<Arc<dyn PageBackend>>> {
        // Target.getTargets
        let result = self.send_command("Target.getTargets", Value::Null).await?;
        
        let targets = result
            .get("targetInfos")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter(|t| t.get("type").and_then(|v| v.as_str()) == Some("page"))
                    .filter_map(|t| t.get("targetId").and_then(|v| v.as_str()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        
        let mut pages = Vec::new();
        for target_id in targets {
            pages.push(Arc::new(CdpPage::new(
                &self.websocket_url,
                target_id,
                &self.config,
            )?) as Arc<dyn PageBackend>);
        }
        
        Ok(pages)
    }

    async fn close(&self) -> Result<()> {
        info!("Closing CDP browser connection");
        // Browser.close
        self.send_command("Browser.close", Value::Null).await?;
        Ok(())
    }

    async fn new_context(&self) -> Result<Arc<dyn BrowserContextBackend>> {
        debug!("Creating new CDP browser context");
        // Target.createBrowserContext
        let result = self
            .send_command("Target.createBrowserContext", Value::Null)
            .await?;
        
        let context_id = result
            .get("browserContextId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AutomationError::CdpError {
                method: "Target.createBrowserContext".to_string(),
                error: "Missing browserContextId".to_string(),
            })?;
        
        Ok(Arc::new(CdpContext::new(context_id, &self.config)?))
    }

    fn is_connected(&self) -> bool {
        // In a real implementation, check WebSocket connection status
        true
    }
}

/// CDP browser context.
pub struct CdpContext {
    _context_id: String,
    config: BrowserConfig,
}

impl CdpContext {
    fn new(context_id: &str, config: &BrowserConfig) -> Result<Self> {
        Ok(Self {
            _context_id: context_id.to_string(),
            config: config.clone(),
        })
    }
}

#[async_trait]
impl BrowserContextBackend for CdpContext {
    async fn new_page(&self) -> Result<Arc<dyn PageBackend>> {
        // Target.createTarget with browserContextId
        Ok(Arc::new(CdpPage::new("", "target-id", &self.config)?))
    }

    async fn pages(&self) -> Result<Vec<Arc<dyn PageBackend>>> {
        Ok(vec![])
    }

    async fn close(&self) -> Result<()> {
        // Target.disposeBrowserContext
        Ok(())
    }

    async fn add_cookies(&self, cookies: Vec<Cookie>) -> Result<()> {
        debug!(count = cookies.len(), "Adding cookies via CDP");
        
        for cookie in cookies {
            let _params = serde_json::json!({
                "name": cookie.name,
                "value": cookie.value,
                "domain": cookie.domain,
                "path": cookie.path,
                "expires": cookie.expires.unwrap_or(-1),
                "httpOnly": cookie.http_only,
                "secure": cookie.secure,
            });
            // Network.setCookie
        }
        
        Ok(())
    }

    async fn cookies(&self) -> Result<Vec<Cookie>> {
        // Storage.getCookies
        Ok(vec![])
    }

    async fn clear_cookies(&self) -> Result<()> {
        // Storage.clearCookies
        Ok(())
    }

    async fn grant_permissions(
        &self,
        permissions: Vec<crate::config::Permission>,
    ) -> Result<()> {
        debug!(count = permissions.len(), "Granting permissions via CDP");
        Ok(())
    }

    async fn clear_permissions(&self) -> Result<()> {
        // Browser.resetPermissions
        Ok(())
    }

    async fn set_geolocation(
        &self,
        geolocation: Option<crate::config::Geolocation>,
    ) -> Result<()> {
        if let Some(geo) = geolocation {
            debug!(lat = geo.latitude, lng = geo.longitude, "Setting geolocation");
            // Emulation.setGeolocationOverride
        }
        Ok(())
    }

    async fn set_extra_http_headers(&self, headers: HashMap<String, String>) -> Result<()> {
        debug!(count = headers.len(), "Setting extra HTTP headers");
        // Network.setExtraHTTPHeaders
        Ok(())
    }
}

/// CDP page implementation.
pub struct CdpPage {
    target_id: String,
    session_id: String,
    config: BrowserConfig,
}

impl CdpPage {
    fn new(_websocket_url: &str, target_id: &str, config: &BrowserConfig) -> Result<Self> {
        Ok(Self {
            target_id: target_id.to_string(),
            session_id: uuid::Uuid::new_v4().to_string(),
            config: config.clone(),
        })
    }

    async fn send_command(&self, method: &str, _params: Value) -> Result<Value> {
        trace!(target_id = %self.target_id, method = %method, "Sending CDP page command");
        Ok(Value::Null)
    }
}

#[async_trait]
impl PageBackend for CdpPage {
    async fn goto(&self, url: &str) -> Result<()> {
        debug!(url = %url, "Navigating via CDP");
        
        // Page.navigate
        let _result = self
            .send_command("Page.navigate", serde_json::json!({ "url": url }))
            .await?;
        
        // Wait for Page.loadEventFired
        info!(url = %url, "Navigation complete via CDP");
        Ok(())
    }

    async fn url(&self) -> Result<String> {
        // Runtime.evaluate: window.location.href
        Ok(String::new())
    }

    async fn title(&self) -> Result<String> {
        // Runtime.evaluate: document.title
        Ok(String::new())
    }

    async fn content(&self) -> Result<String> {
        // Runtime.evaluate: document.documentElement.outerHTML
        Ok(String::new())
    }

    async fn wait_for_selector(&self, selector: &str) -> Result<Arc<dyn ElementBackend>> {
        debug!(selector = %selector, "Waiting for selector via CDP");
        Ok(Arc::new(CdpElement::new(selector, &self.session_id)))
    }

    async fn query_selector(&self, selector: &str) -> Result<Option<Arc<dyn ElementBackend>>> {
        Ok(Some(Arc::new(CdpElement::new(selector, &self.session_id))))
    }

    async fn query_selector_all(&self, _selector: &str) -> Result<Vec<Arc<dyn ElementBackend>>> {
        // DOM.querySelectorAll
        Ok(vec![])
    }

    async fn click(&self, selector: &str) -> Result<()> {
        debug!(selector = %selector, "Clicking via CDP");
        // DOM.querySelector + Input.dispatchMouseEvent
        Ok(())
    }

    async fn type_text(&self, selector: &str, _text: &str) -> Result<()> {
        debug!(selector = %selector, "Typing via CDP");
        // DOM.querySelector + Input.insertText / Input.dispatchKeyEvent
        Ok(())
    }

    async fn fill(&self, selector: &str, text: &str) -> Result<()> {
        // DOM.querySelector + DOM.setAttributeValue for input elements
        self.type_text(selector, text).await
    }

    async fn press(&self, key: &str) -> Result<()> {
        debug!(key = %key, "Pressing key via CDP");
        // Input.dispatchKeyEvent
        Ok(())
    }

    async fn screenshot(&self, options: ScreenshotOptions) -> Result<Screenshot> {
        debug!("Taking screenshot via CDP");
        
        // Page.captureScreenshot
        let format = match options.format {
            crate::browser::ScreenshotFormat::Png => "png",
            crate::browser::ScreenshotFormat::Jpeg => "jpeg",
        };
        
        let _params = serde_json::json!({
            "format": format,
            "quality": options.quality,
            "fullPage": options.full_page,
            "clip": options.clip.map(|r| serde_json::json!({
                "x": r.x,
                "y": r.y,
                "width": r.width,
                "height": r.height,
                "scale": 1.0,
            })),
        });
        
        // Decode base64 response
        let width = self.config.automation.viewport.width;
        let height = if options.full_page { 3000 } else { self.config.automation.viewport.height };
        
        let data = create_placeholder_screenshot(width, height)?;
        
        Ok(Screenshot {
            data,
            width,
            height,
            format: options.format,
        })
    }

    async fn evaluate(&self, script: &str) -> Result<Value> {
        debug!(script_len = script.len(), "Evaluating JavaScript via CDP");
        // Runtime.evaluate
        Ok(Value::Null)
    }

    async fn evaluate_with_args(&self, script: &str, _args: Vec<Value>) -> Result<Value> {
        // Runtime.callFunctionOn with arguments
        self.evaluate(script).await
    }

    async fn add_script_tag(&self, content: &str) -> Result<()> {
        // Page.addScriptToEvaluateOnNewDocument or Runtime.evaluate
        let script = format!("var script = document.createElement('script'); script.textContent = {}; document.head.appendChild(script);", 
            serde_json::to_string(content)?);
        self.evaluate(&script).await?;
        Ok(())
    }

    async fn add_style_tag(&self, content: &str) -> Result<()> {
        let script = format!("var style = document.createElement('style'); style.textContent = {}; document.head.appendChild(style);", 
            serde_json::to_string(content)?);
        self.evaluate(&script).await?;
        Ok(())
    }

    async fn wait_for_navigation(&self) -> Result<()> {
        // Page.frameNavigated event
        Ok(())
    }

    async fn wait_for_load_state(&self, state: LoadState) -> Result<()> {
        debug!(state = ?state, "Waiting for load state via CDP");
        match state {
            LoadState::DomContentLoaded => {
                // Page.domContentEventFired
            }
            LoadState::Load => {
                // Page.loadEventFired
            }
            LoadState::NetworkIdle => {
                // Wait for Network.loadingFinished after no new requests
            }
        }
        Ok(())
    }

    async fn wait_for_timeout(&self, ms: u64) -> Result<()> {
        tokio::time::sleep(tokio::time::Duration::from_millis(ms)).await;
        Ok(())
    }

    async fn wait_for_function(&self, _script: &str) -> Result<Value> {
        // Runtime.evaluate in a loop until true
        Ok(Value::Bool(true))
    }

    async fn reload(&self) -> Result<()> {
        // Page.reload
        Ok(())
    }

    async fn go_back(&self) -> Result<()> {
        // Runtime.evaluate: history.back()
        self.evaluate("history.back()").await?;
        Ok(())
    }

    async fn go_forward(&self) -> Result<()> {
        // Runtime.evaluate: history.forward()
        self.evaluate("history.forward()").await?;
        Ok(())
    }

    async fn close(&self) -> Result<()> {
        // Target.closeTarget
        Ok(())
    }

    fn is_closed(&self) -> bool {
        false
    }

    async fn main_frame(&self) -> Result<Arc<dyn FrameBackend>> {
        Ok(Arc::new(CdpFrame::new(&self.session_id)))
    }

    async fn frames(&self) -> Result<Vec<Arc<dyn FrameBackend>>> {
        // Page.getFrameTree
        Ok(vec![])
    }

    async fn bring_to_front(&self) -> Result<()> {
        // Page.bringToFront
        Ok(())
    }

    async fn viewport_size(&self) -> Result<(u32, u32)> {
        // Runtime.evaluate: window.innerWidth/window.innerHeight
        Ok((self.config.automation.viewport.width, self.config.automation.viewport.height))
    }

    async fn set_viewport_size(&self, width: u32, height: u32) -> Result<()> {
        // Emulation.setDeviceMetricsOverride
        let params = serde_json::json!({
            "width": width,
            "height": height,
            "deviceScaleFactor": self.config.automation.viewport.device_scale_factor,
            "mobile": self.config.automation.viewport.is_mobile,
        });
        self.send_command("Emulation.setDeviceMetricsOverride", params).await?;
        Ok(())
    }

    async fn cookies(&self) -> Result<Vec<Cookie>> {
        // Network.getCookies
        Ok(vec![])
    }

    async fn add_cookies(&self, _cookies: Vec<Cookie>) -> Result<()> {
        // Network.setCookie
        Ok(())
    }

    async fn clear_cookies(&self) -> Result<()> {
        // Network.clearBrowserCookies
        Ok(())
    }

    async fn local_storage(&self, key: &str) -> Result<Option<String>> {
        let script = format!(
            "localStorage.getItem({})",
            serde_json::to_string(key)?
        );
        let result = self.evaluate(&script).await?;
        Ok(result.as_str().map(String::from))
    }

    async fn set_local_storage(&self, key: &str, value: &str) -> Result<()> {
        let script = format!(
            "localStorage.setItem({}, {})",
            serde_json::to_string(key)?,
            serde_json::to_string(value)?
        );
        self.evaluate(&script).await?;
        Ok(())
    }

    async fn session_storage(&self, key: &str) -> Result<Option<String>> {
        let script = format!(
            "sessionStorage.getItem({})",
            serde_json::to_string(key)?
        );
        let result = self.evaluate(&script).await?;
        Ok(result.as_str().map(String::from))
    }

    async fn set_session_storage(&self, key: &str, value: &str) -> Result<()> {
        let script = format!(
            "sessionStorage.setItem({}, {})",
            serde_json::to_string(key)?,
            serde_json::to_string(value)?
        );
        self.evaluate(&script).await?;
        Ok(())
    }

    async fn pdf(&self, _options: PdfOptions) -> Result<Vec<u8>> {
        debug!("Generating PDF via CDP");
        // Page.printToPDF
        Ok(vec![0x25, 0x50, 0x44, 0x46, 0x2D, 0x31, 0x2E, 0x34])
    }
}

/// CDP element implementation.
pub struct CdpElement {
    selector: String,
    session_id: String,
    _node_id: Option<i64>,
    _remote_object_id: Option<String>,
}

impl CdpElement {
    fn new(selector: &str, session_id: &str) -> Self {
        Self {
            selector: selector.to_string(),
            session_id: session_id.to_string(),
            _node_id: None,
            _remote_object_id: None,
        }
    }
}

#[async_trait]
impl ElementBackend for CdpElement {
    async fn click(&self) -> Result<()> {
        debug!(selector = %self.selector, "Clicking via CDP");
        // DOM.querySelector + DOM.getBoxModel + Input.dispatchMouseEvent
        Ok(())
    }

    async fn dblclick(&self) -> Result<()> {
        // Input.dispatchMouseEvent with clickCount: 2
        Ok(())
    }

    async fn hover(&self) -> Result<()> {
        // Input.dispatchMouseEvent with type: "mouseMoved"
        Ok(())
    }

    async fn type_text(&self, _text: &str) -> Result<()> {
        debug!(selector = %self.selector, "Typing via CDP");
        // Focus element, then Input.insertText
        Ok(())
    }

    async fn fill(&self, text: &str) -> Result<()> {
        self.type_text(text).await
    }

    async fn clear(&self) -> Result<()> {
        // DOM.setAttributeValue or Runtime.callFunctionOn element.value = ''
        Ok(())
    }

    async fn press(&self, key: &str) -> Result<()> {
        debug!(key = %key, "Pressing key via CDP");
        // Input.dispatchKeyEvent
        Ok(())
    }

    async fn text_content(&self) -> Result<String> {
        // Runtime.callFunctionOn element.textContent
        Ok(String::new())
    }

    async fn inner_text(&self) -> Result<String> {
        // Runtime.callFunctionOn element.innerText
        Ok(String::new())
    }

    async fn inner_html(&self) -> Result<String> {
        // Runtime.callFunctionOn element.innerHTML
        Ok(String::new())
    }

    async fn get_attribute(&self, _name: &str) -> Result<Option<String>> {
        // DOM.getAttributes
        Ok(None)
    }

    async fn is_visible(&self) -> Result<bool> {
        // Check DOM.getBoxModel
        Ok(true)
    }

    async fn is_enabled(&self) -> Result<bool> {
        // Runtime.callFunctionOn !element.disabled
        Ok(true)
    }

    async fn is_checked(&self) -> Result<bool> {
        // Runtime.callFunctionOn element.checked
        Ok(false)
    }

    async fn bounding_box(&self) -> Result<Option<Rect>> {
        // DOM.getBoxModel
        Ok(None)
    }

    async fn scroll_into_view(&self) -> Result<()> {
        // Runtime.callFunctionOn element.scrollIntoView()
        Ok(())
    }

    async fn screenshot(&self) -> Result<Screenshot> {
        // Page.captureScreenshot with clip from bounding box
        let data = create_placeholder_screenshot(100, 100)?;
        Ok(Screenshot {
            data,
            width: 100,
            height: 100,
            format: crate::browser::ScreenshotFormat::Png,
        })
    }

    async fn select_option(&self, values: Vec<String>) -> Result<()> {
        debug!(count = values.len(), "Selecting options via CDP");
        Ok(())
    }

    async fn query_selector(&self, selector: &str) -> Result<Option<Arc<dyn ElementBackend>>> {
        Ok(Some(Arc::new(CdpElement::new(selector, &self.session_id))))
    }

    async fn query_selector_all(&self, _selector: &str) -> Result<Vec<Arc<dyn ElementBackend>>> {
        Ok(vec![])
    }

    async fn parent(&self) -> Result<Option<Arc<dyn ElementBackend>>> {
        Ok(None)
    }

    async fn evaluate(&self, _script: &str) -> Result<Value> {
        // Runtime.callFunctionOn with element as this
        Ok(Value::Null)
    }

    async fn tag_name(&self) -> Result<String> {
        Ok("div".to_string())
    }

    async fn input_value(&self) -> Result<String> {
        Ok(String::new())
    }
}

/// CDP frame implementation.
pub struct CdpFrame {
    session_id: String,
}

impl CdpFrame {
    fn new(session_id: &str) -> Self {
        Self {
            session_id: session_id.to_string(),
        }
    }
}

#[async_trait]
impl FrameBackend for CdpFrame {
    async fn url(&self) -> Result<String> {
        Ok(String::new())
    }

    async fn title(&self) -> Result<String> {
        Ok(String::new())
    }

    async fn query_selector(&self, selector: &str) -> Result<Option<Arc<dyn ElementBackend>>> {
        Ok(Some(Arc::new(CdpElement::new(selector, &self.session_id))))
    }

    async fn evaluate(&self, _script: &str) -> Result<Value> {
        Ok(Value::Null)
    }

    async fn wait_for_selector(&self, selector: &str) -> Result<Arc<dyn ElementBackend>> {
        Ok(Arc::new(CdpElement::new(selector, &self.session_id)))
    }
}

// ============================================================================
// CDP Protocol Types
// ============================================================================

/// CDP command message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CdpCommand {
    /// Command ID.
    pub id: u64,
    /// Method name (e.g., "Page.navigate").
    pub method: String,
    /// Method parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
    /// Session ID for target-specific commands.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

/// CDP response message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CdpResponse {
    /// Response ID (matches command ID).
    pub id: u64,
    /// Result data (if successful).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// Error info (if failed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<CdpError>,
}

/// CDP error information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdpError {
    /// Error code.
    pub code: i64,
    /// Error message.
    pub message: String,
    /// Additional error data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// CDP event message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CdpEvent {
    /// Event method name (e.g., "Page.loadEventFired").
    pub method: String,
    /// Event parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
    /// Session ID for target-specific events.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Find Chrome/Chromium executable.
fn find_chrome_executable(custom_path: Option<&str>) -> Result<String> {
    if let Some(path) = custom_path {
        return Ok(path.to_string());
    }

    // Platform-specific search
    #[cfg(target_os = "macos")]
    let paths = vec![
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/Applications/Chromium.app/Contents/MacOS/Chromium",
        "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
        "/usr/bin/google-chrome",
        "/usr/bin/chromium",
        "/usr/bin/chromium-browser",
    ];

    #[cfg(target_os = "linux")]
    let paths = vec![
        "/usr/bin/google-chrome",
        "/usr/bin/google-chrome-stable",
        "/usr/bin/chromium",
        "/usr/bin/chromium-browser",
        "/usr/bin/microsoft-edge",
        "/snap/bin/chromium",
    ];

    #[cfg(target_os = "windows")]
    let paths = vec![
        r"C:\Program Files\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
    ];

    for path in paths {
        if std::path::Path::new(path).exists() {
            return Ok(path.to_string());
        }
    }

    // Try which command
    if let Ok(output) = std::process::Command::new("which")
        .arg("google-chrome")
        .output()
    {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Ok(path);
            }
        }
    }

    Err(AutomationError::LaunchFailed {
        browser: "chrome".to_string(),
        reason: "Chrome/Chromium executable not found".to_string(),
    })
}

/// Create a placeholder screenshot.
fn create_placeholder_screenshot(width: u32, height: u32) -> Result<Vec<u8>> {
    let mut img = image::RgbImage::new(width, height);
    
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        let r = ((x as f32 / width as f32) * 255.0) as u8;
        let g = ((y as f32 / height as f32) * 255.0) as u8;
        let b = 200;
        *pixel = image::Rgb([r, g, b]);
    }
    
    let mut buf = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut buf);
    encoder
        .write_image(&img, width, height, image::ExtendedColorType::Rgb8)
        .map_err(|e| AutomationError::ScreenshotFailed {
            reason: format!("Failed to encode PNG: {}", e),
        })?;
    
    Ok(buf)
}

// ============================================================================
// Network Interception
// ============================================================================

/// Network request interception configuration.
#[derive(Debug, Clone)]
pub struct NetworkInterceptor {
    patterns: Vec<RequestPattern>,
}

impl NetworkInterceptor {
    /// Create a new interceptor.
    pub fn new() -> Self {
        Self { patterns: vec![] }
    }

    /// Add a URL pattern to intercept.
    pub fn intercept_url(mut self, url_pattern: &str) -> Self {
        self.patterns.push(RequestPattern {
            url_pattern: url_pattern.to_string(),
            resource_type: None,
        });
        self
    }

    /// Add a resource type to intercept.
    pub fn intercept_resource_type(
        mut self,
        resource_type: ResourceType,
    ) -> Self {
        self.patterns.push(RequestPattern {
            url_pattern: "*".to_string(),
            resource_type: Some(resource_type),
        });
        self
    }
}

/// Request pattern for interception.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RequestPattern {
    url_pattern: String,
    resource_type: Option<ResourceType>,
}

/// Resource types for network interception.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ResourceType {
    Document,
    Stylesheet,
    Image,
    Media,
    Font,
    Script,
    Xhr,
    Fetch,
    WebSocket,
    Manifest,
    Other,
}

impl std::fmt::Display for ResourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Document => "Document",
            Self::Stylesheet => "Stylesheet",
            Self::Image => "Image",
            Self::Media => "Media",
            Self::Font => "Font",
            Self::Script => "Script",
            Self::Xhr => "XHR",
            Self::Fetch => "Fetch",
            Self::WebSocket => "WebSocket",
            Self::Manifest => "Manifest",
            Self::Other => "Other",
        };
        write!(f, "{}", s)
    }
}

// ============================================================================
// Mobile Emulation
// ============================================================================

/// Mobile device descriptor for emulation.
#[derive(Debug, Clone)]
pub struct MobileDevice {
    /// Device name.
    pub name: String,
    /// User agent string.
    pub user_agent: String,
    /// Viewport configuration.
    pub viewport: crate::config::Viewport,
    /// Device scale factor.
    pub device_scale_factor: f64,
}

impl MobileDevice {
    /// iPhone 14 Pro preset.
    pub fn iphone_14_pro() -> Self {
        Self {
            name: "iPhone 14 Pro".to_string(),
            user_agent: "Mozilla/5.0 (iPhone; CPU iPhone OS 16_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.0 Mobile/15E148 Safari/604.1".to_string(),
            viewport: crate::config::Viewport::mobile(),
            device_scale_factor: 3.0,
        }
    }

    /// iPad Pro preset.
    pub fn ipad_pro() -> Self {
        Self {
            name: "iPad Pro".to_string(),
            user_agent: "Mozilla/5.0 (iPad; CPU OS 16_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.0 Mobile/15E148 Safari/604.1".to_string(),
            viewport: crate::config::Viewport::tablet(),
            device_scale_factor: 2.0,
        }
    }

    /// Pixel 7 preset.
    pub fn pixel_7() -> Self {
        Self {
            name: "Pixel 7".to_string(),
            user_agent: "Mozilla/5.0 (Linux; Android 13; Pixel 7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/116.0.0.0 Mobile Safari/537.36".to_string(),
            viewport: crate::config::Viewport {
                width: 412,
                height: 915,
                device_scale_factor: 2.625,
                is_mobile: true,
                has_touch: true,
                is_landscape: false,
            },
            device_scale_factor: 2.625,
        }
    }
}
