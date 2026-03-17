//! High-level browser orchestration.
//!
//! This module provides a unified interface for browser automation,
//! abstracting over different backends (Playwright, CDP, etc.).

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::BrowserConfig;
use crate::error::Result;

/// A browser instance that can create pages and manage sessions.
pub struct Browser {
    pub(crate) inner: Arc<dyn BrowserBackend>,
    pub(crate) config: BrowserConfig,
}

impl Browser {
    /// Create a new browser instance with the given configuration.
    pub async fn new(config: BrowserConfig) -> Result<Self> {
        let backend: Arc<dyn BrowserBackend> = match config.automation.browser {
            #[cfg(feature = "playwright")]
            _ => Arc::new(crate::playwright::PlaywrightBackend::new(&config).await?),
            #[cfg(all(feature = "cdp", not(feature = "playwright")))]
            BrowserType::Chromium | BrowserType::Edge => {
                Arc::new(crate::cdp::CdpBackend::new(&config).await?)
            }
            #[cfg(not(any(feature = "playwright", feature = "cdp")))]
            _ => {
                return Err(AutomationError::ConfigError {
                    field: "browser".to_string(),
                    reason: "No browser backend enabled. Enable 'playwright' or 'cdp' feature."
                        .to_string(),
                });
            }
        };

        Ok(Self {
            inner: backend,
            config,
        })
    }

    /// Create a new browser page (tab).
    pub async fn new_page(&self) -> Result<Page> {
        let inner = self.inner.new_page().await?;
        Ok(Page { inner })
    }

    /// Navigate to a URL and return a page.
    pub async fn navigate(&self, url: &str) -> Result<Page> {
        let page = self.new_page().await?;
        page.goto(url).await?;
        Ok(page)
    }

    /// Get all open pages.
    pub async fn pages(&self) -> Result<Vec<Page>> {
        let inners = self.inner.pages().await?;
        Ok(inners.into_iter().map(|inner| Page { inner }).collect())
    }

    /// Close the browser and all its pages.
    pub async fn close(&self) -> Result<()> {
        self.inner.close().await
    }

    /// Get the browser configuration.
    pub fn config(&self) -> &BrowserConfig {
        &self.config
    }

    /// Create a new browser context (isolated session).
    pub async fn new_context(&self) -> Result<BrowserContext> {
        let inner = self.inner.new_context().await?;
        Ok(BrowserContext { inner })
    }

    /// Check if the browser is still connected.
    pub fn is_connected(&self) -> bool {
        self.inner.is_connected()
    }
}

/// A browser context provides an isolated environment (cookies, localStorage, etc.).
pub struct BrowserContext {
    inner: Arc<dyn BrowserContextBackend>,
}

impl BrowserContext {
    /// Create a new page in this context.
    pub async fn new_page(&self) -> Result<Page> {
        let inner = self.inner.new_page().await?;
        Ok(Page { inner })
    }

    /// Get all pages in this context.
    pub async fn pages(&self) -> Result<Vec<Page>> {
        let inners = self.inner.pages().await?;
        Ok(inners.into_iter().map(|inner| Page { inner }).collect())
    }

    /// Close this context.
    pub async fn close(&self) -> Result<()> {
        self.inner.close().await
    }

    /// Add cookies to this context.
    pub async fn add_cookies(&self, cookies: Vec<Cookie>) -> Result<()> {
        self.inner.add_cookies(cookies).await
    }

    /// Get all cookies from this context.
    pub async fn cookies(&self) -> Result<Vec<Cookie>> {
        self.inner.cookies().await
    }

    /// Clear all cookies.
    pub async fn clear_cookies(&self) -> Result<()> {
        self.inner.clear_cookies().await
    }

    /// Grant permissions to this context.
    pub async fn grant_permissions(
        &self,
        permissions: Vec<crate::config::Permission>,
    ) -> Result<()> {
        self.inner.grant_permissions(permissions).await
    }

    /// Clear all permissions.
    pub async fn clear_permissions(&self) -> Result<()> {
        self.inner.clear_permissions().await
    }

    /// Set geolocation for this context.
    pub async fn set_geolocation(
        &self,
        geolocation: Option<crate::config::Geolocation>,
    ) -> Result<()> {
        self.inner.set_geolocation(geolocation).await
    }

    /// Set extra HTTP headers for this context.
    pub async fn set_extra_http_headers(&self, headers: HashMap<String, String>) -> Result<()> {
        self.inner.set_extra_http_headers(headers).await
    }
}

/// A single page/tab in a browser.
pub struct Page {
    inner: Arc<dyn PageBackend>,
}

impl Page {
    /// Navigate to a URL.
    pub async fn goto(&self, url: &str) -> Result<()> {
        self.inner.goto(url).await
    }

    /// Get the current URL.
    pub async fn url(&self) -> Result<String> {
        self.inner.url().await
    }

    /// Get the page title.
    pub async fn title(&self) -> Result<String> {
        self.inner.title().await
    }

    /// Get the page content (HTML).
    pub async fn content(&self) -> Result<String> {
        self.inner.content().await
    }

    /// Wait for an element matching the selector to appear.
    pub async fn wait_for_selector(&self, selector: &str) -> Result<Element> {
        let inner = self.inner.wait_for_selector(selector).await?;
        Ok(Element { inner })
    }

    /// Query for a single element.
    pub async fn query_selector(&self, selector: &str) -> Result<Option<Element>> {
        match self.inner.query_selector(selector).await? {
            Some(inner) => Ok(Some(Element { inner })),
            None => Ok(None),
        }
    }

    /// Query for all elements matching the selector.
    pub async fn query_selector_all(&self, selector: &str) -> Result<Vec<Element>> {
        let inners = self.inner.query_selector_all(selector).await?;
        Ok(inners.into_iter().map(|inner| Element { inner }).collect())
    }

    /// Click on an element.
    pub async fn click(&self, selector: &str) -> Result<()> {
        self.inner.click(selector).await
    }

    /// Type text into an element.
    pub async fn type_text(&self, selector: &str, text: &str) -> Result<()> {
        self.inner.type_text(selector, text).await
    }

    /// Fill a form field (clear first, then type).
    pub async fn fill(&self, selector: &str, text: &str) -> Result<()> {
        self.inner.fill(selector, text).await
    }

    /// Press a key.
    pub async fn press(&self, key: &str) -> Result<()> {
        self.inner.press(key).await
    }

    /// Take a screenshot of the page.
    pub async fn screenshot(&self) -> Result<Screenshot> {
        self.screenshot_with_options(ScreenshotOptions::default())
            .await
    }

    /// Take a screenshot with options.
    pub async fn screenshot_with_options(&self, options: ScreenshotOptions) -> Result<Screenshot> {
        self.inner.screenshot(options).await
    }

    /// Execute JavaScript on the page.
    pub async fn evaluate(&self, script: &str) -> Result<Value> {
        self.inner.evaluate(script).await
    }

    /// Execute JavaScript with arguments.
    pub async fn evaluate_with_args(&self, script: &str, args: Vec<Value>) -> Result<Value> {
        self.inner.evaluate_with_args(script, args).await
    }

    /// Add a script tag to the page.
    pub async fn add_script_tag(&self, content: &str) -> Result<()> {
        self.inner.add_script_tag(content).await
    }

    /// Add a style tag to the page.
    pub async fn add_style_tag(&self, content: &str) -> Result<()> {
        self.inner.add_style_tag(content).await
    }

    /// Wait for navigation to complete.
    pub async fn wait_for_navigation(&self) -> Result<()> {
        self.inner.wait_for_navigation().await
    }

    /// Wait for a specific load state.
    pub async fn wait_for_load_state(&self, state: LoadState) -> Result<()> {
        self.inner.wait_for_load_state(state).await
    }

    /// Wait for a timeout.
    pub async fn wait_for_timeout(&self, ms: u64) -> Result<()> {
        self.inner.wait_for_timeout(ms).await
    }

    /// Wait for a function to return true.
    pub async fn wait_for_function(&self, script: &str) -> Result<Value> {
        self.inner.wait_for_function(script).await
    }

    /// Reload the page.
    pub async fn reload(&self) -> Result<()> {
        self.inner.reload().await
    }

    /// Go back in history.
    pub async fn go_back(&self) -> Result<()> {
        self.inner.go_back().await
    }

    /// Go forward in history.
    pub async fn go_forward(&self) -> Result<()> {
        self.inner.go_forward().await
    }

    /// Close the page.
    pub async fn close(&self) -> Result<()> {
        self.inner.close().await
    }

    /// Check if the page is closed.
    pub fn is_closed(&self) -> bool {
        self.inner.is_closed()
    }

    /// Get the page's main frame.
    pub async fn main_frame(&self) -> Result<Frame> {
        let inner = self.inner.main_frame().await?;
        Ok(Frame { inner })
    }

    /// Get all frames in the page.
    pub async fn frames(&self) -> Result<Vec<Frame>> {
        let inners = self.inner.frames().await?;
        Ok(inners.into_iter().map(|inner| Frame { inner }).collect())
    }

    /// Bring this page to front (activate tab).
    pub async fn bring_to_front(&self) -> Result<()> {
        self.inner.bring_to_front().await
    }

    /// Get page viewport size.
    pub async fn viewport_size(&self) -> Result<(u32, u32)> {
        self.inner.viewport_size().await
    }

    /// Set page viewport size.
    pub async fn set_viewport_size(&self, width: u32, height: u32) -> Result<()> {
        self.inner.set_viewport_size(width, height).await
    }

    /// Get all cookies for this page.
    pub async fn cookies(&self) -> Result<Vec<Cookie>> {
        self.inner.cookies().await
    }

    /// Add cookies.
    pub async fn add_cookies(&self, cookies: Vec<Cookie>) -> Result<()> {
        self.inner.add_cookies(cookies).await
    }

    /// Clear cookies.
    pub async fn clear_cookies(&self) -> Result<()> {
        self.inner.clear_cookies().await
    }

    /// Get localStorage item.
    pub async fn local_storage(&self, key: &str) -> Result<Option<String>> {
        self.inner.local_storage(key).await
    }

    /// Set localStorage item.
    pub async fn set_local_storage(&self, key: &str, value: &str) -> Result<()> {
        self.inner.set_local_storage(key, value).await
    }

    /// Get sessionStorage item.
    pub async fn session_storage(&self, key: &str) -> Result<Option<String>> {
        self.inner.session_storage(key).await
    }

    /// Set sessionStorage item.
    pub async fn set_session_storage(&self, key: &str, value: &str) -> Result<()> {
        self.inner.set_session_storage(key, value).await
    }

    /// Generate PDF of the page.
    pub async fn pdf(&self, options: PdfOptions) -> Result<Vec<u8>> {
        self.inner.pdf(options).await
    }
}

/// A single DOM element.
pub struct Element {
    inner: Arc<dyn ElementBackend>,
}

impl Element {
    /// Click on this element.
    pub async fn click(&self) -> Result<()> {
        self.inner.click().await
    }

    /// Double-click on this element.
    pub async fn dblclick(&self) -> Result<()> {
        self.inner.dblclick().await
    }

    /// Hover over this element.
    pub async fn hover(&self) -> Result<()> {
        self.inner.hover().await
    }

    /// Type text into this element.
    pub async fn type_text(&self, text: &str) -> Result<()> {
        self.inner.type_text(text).await
    }

    /// Fill this element (clear first, then type).
    pub async fn fill(&self, text: &str) -> Result<()> {
        self.inner.fill(text).await
    }

    /// Clear this element's value.
    pub async fn clear(&self) -> Result<()> {
        self.inner.clear().await
    }

    /// Press a key while this element is focused.
    pub async fn press(&self, key: &str) -> Result<()> {
        self.inner.press(key).await
    }

    /// Get the element's text content.
    pub async fn text_content(&self) -> Result<String> {
        self.inner.text_content().await
    }

    /// Get the element's inner text (visible text only).
    pub async fn inner_text(&self) -> Result<String> {
        self.inner.inner_text().await
    }

    /// Get the element's inner HTML.
    pub async fn inner_html(&self) -> Result<String> {
        self.inner.inner_html().await
    }

    /// Get an attribute value.
    pub async fn get_attribute(&self, name: &str) -> Result<Option<String>> {
        self.inner.get_attribute(name).await
    }

    /// Check if the element is visible.
    pub async fn is_visible(&self) -> Result<bool> {
        self.inner.is_visible().await
    }

    /// Check if the element is enabled.
    pub async fn is_enabled(&self) -> Result<bool> {
        self.inner.is_enabled().await
    }

    /// Check if the element is checked (for checkboxes/radio buttons).
    pub async fn is_checked(&self) -> Result<bool> {
        self.inner.is_checked().await
    }

    /// Get the element's bounding box.
    pub async fn bounding_box(&self) -> Result<Option<Rect>> {
        self.inner.bounding_box().await
    }

    /// Scroll the element into view.
    pub async fn scroll_into_view(&self) -> Result<()> {
        self.inner.scroll_into_view().await
    }

    /// Take a screenshot of this element.
    pub async fn screenshot(&self) -> Result<Screenshot> {
        self.inner.screenshot().await
    }

    /// Select options in a `<select>` element.
    pub async fn select_option(&self, values: Vec<String>) -> Result<()> {
        self.inner.select_option(values).await
    }

    /// Query for a child element.
    pub async fn query_selector(&self, selector: &str) -> Result<Option<Element>> {
        match self.inner.query_selector(selector).await? {
            Some(inner) => Ok(Some(Element { inner })),
            None => Ok(None),
        }
    }

    /// Query for all child elements.
    pub async fn query_selector_all(&self, selector: &str) -> Result<Vec<Element>> {
        let inners = self.inner.query_selector_all(selector).await?;
        Ok(inners.into_iter().map(|inner| Element { inner }).collect())
    }

    /// Get the parent element.
    pub async fn parent(&self) -> Result<Option<Element>> {
        match self.inner.parent().await? {
            Some(inner) => Ok(Some(Element { inner })),
            None => Ok(None),
        }
    }

    /// Evaluate JavaScript with this element as the argument.
    pub async fn evaluate(&self, script: &str) -> Result<Value> {
        self.inner.evaluate(script).await
    }

    /// Get the element's tag name.
    pub async fn tag_name(&self) -> Result<String> {
        self.inner.tag_name().await
    }

    /// Get the element's input value.
    pub async fn input_value(&self) -> Result<String> {
        self.inner.input_value().await
    }
}

/// A frame (iframe) within a page.
pub struct Frame {
    inner: Arc<dyn FrameBackend>,
}

impl Frame {
    /// Get the frame's URL.
    pub async fn url(&self) -> Result<String> {
        self.inner.url().await
    }

    /// Get the frame's title.
    pub async fn title(&self) -> Result<String> {
        self.inner.title().await
    }

    /// Query for an element within the frame.
    pub async fn query_selector(&self, selector: &str) -> Result<Option<Element>> {
        match self.inner.query_selector(selector).await? {
            Some(inner) => Ok(Some(Element { inner })),
            None => Ok(None),
        }
    }

    /// Execute JavaScript within the frame.
    pub async fn evaluate(&self, script: &str) -> Result<Value> {
        self.inner.evaluate(script).await
    }

    /// Wait for an element within the frame.
    pub async fn wait_for_selector(&self, selector: &str) -> Result<Element> {
        let inner = self.inner.wait_for_selector(selector).await?;
        Ok(Element { inner })
    }
}

// ============================================================================
// Backend traits (internal)
// ============================================================================

#[async_trait]
pub(crate) trait BrowserBackend: Send + Sync {
    async fn new_page(&self) -> Result<Arc<dyn PageBackend>>;
    async fn pages(&self) -> Result<Vec<Arc<dyn PageBackend>>>;
    async fn close(&self) -> Result<()>;
    async fn new_context(&self) -> Result<Arc<dyn BrowserContextBackend>>;
    fn is_connected(&self) -> bool;
}

#[async_trait]
pub(crate) trait BrowserContextBackend: Send + Sync {
    async fn new_page(&self) -> Result<Arc<dyn PageBackend>>;
    async fn pages(&self) -> Result<Vec<Arc<dyn PageBackend>>>;
    async fn close(&self) -> Result<()>;
    async fn add_cookies(&self, cookies: Vec<Cookie>) -> Result<()>;
    async fn cookies(&self) -> Result<Vec<Cookie>>;
    async fn clear_cookies(&self) -> Result<()>;
    async fn grant_permissions(&self, permissions: Vec<crate::config::Permission>) -> Result<()>;
    async fn clear_permissions(&self) -> Result<()>;
    async fn set_geolocation(&self, geolocation: Option<crate::config::Geolocation>) -> Result<()>;
    async fn set_extra_http_headers(&self, headers: HashMap<String, String>) -> Result<()>;
}

#[async_trait]
pub(crate) trait PageBackend: Send + Sync {
    async fn goto(&self, url: &str) -> Result<()>;
    async fn url(&self) -> Result<String>;
    async fn title(&self) -> Result<String>;
    async fn content(&self) -> Result<String>;
    async fn wait_for_selector(&self, selector: &str) -> Result<Arc<dyn ElementBackend>>;
    async fn query_selector(&self, selector: &str) -> Result<Option<Arc<dyn ElementBackend>>>;
    async fn query_selector_all(&self, selector: &str) -> Result<Vec<Arc<dyn ElementBackend>>>;
    async fn click(&self, selector: &str) -> Result<()>;
    async fn type_text(&self, selector: &str, text: &str) -> Result<()>;
    async fn fill(&self, selector: &str, text: &str) -> Result<()>;
    async fn press(&self, key: &str) -> Result<()>;
    async fn screenshot(&self, options: ScreenshotOptions) -> Result<Screenshot>;
    async fn evaluate(&self, script: &str) -> Result<Value>;
    async fn evaluate_with_args(&self, script: &str, args: Vec<Value>) -> Result<Value>;
    async fn add_script_tag(&self, content: &str) -> Result<()>;
    async fn add_style_tag(&self, content: &str) -> Result<()>;
    async fn wait_for_navigation(&self) -> Result<()>;
    async fn wait_for_load_state(&self, state: LoadState) -> Result<()>;
    async fn wait_for_timeout(&self, ms: u64) -> Result<()>;
    async fn wait_for_function(&self, script: &str) -> Result<Value>;
    async fn reload(&self) -> Result<()>;
    async fn go_back(&self) -> Result<()>;
    async fn go_forward(&self) -> Result<()>;
    async fn close(&self) -> Result<()>;
    fn is_closed(&self) -> bool;
    async fn main_frame(&self) -> Result<Arc<dyn FrameBackend>>;
    async fn frames(&self) -> Result<Vec<Arc<dyn FrameBackend>>>;
    async fn bring_to_front(&self) -> Result<()>;
    async fn viewport_size(&self) -> Result<(u32, u32)>;
    async fn set_viewport_size(&self, width: u32, height: u32) -> Result<()>;
    async fn cookies(&self) -> Result<Vec<Cookie>>;
    async fn add_cookies(&self, cookies: Vec<Cookie>) -> Result<()>;
    async fn clear_cookies(&self) -> Result<()>;
    async fn local_storage(&self, key: &str) -> Result<Option<String>>;
    async fn set_local_storage(&self, key: &str, value: &str) -> Result<()>;
    async fn session_storage(&self, key: &str) -> Result<Option<String>>;
    async fn set_session_storage(&self, key: &str, value: &str) -> Result<()>;
    async fn pdf(&self, options: PdfOptions) -> Result<Vec<u8>>;
}

#[async_trait]
pub(crate) trait ElementBackend: Send + Sync {
    async fn click(&self) -> Result<()>;
    async fn dblclick(&self) -> Result<()>;
    async fn hover(&self) -> Result<()>;
    async fn type_text(&self, text: &str) -> Result<()>;
    async fn fill(&self, text: &str) -> Result<()>;
    async fn clear(&self) -> Result<()>;
    async fn press(&self, key: &str) -> Result<()>;
    async fn text_content(&self) -> Result<String>;
    async fn inner_text(&self) -> Result<String>;
    async fn inner_html(&self) -> Result<String>;
    async fn get_attribute(&self, name: &str) -> Result<Option<String>>;
    async fn is_visible(&self) -> Result<bool>;
    async fn is_enabled(&self) -> Result<bool>;
    async fn is_checked(&self) -> Result<bool>;
    async fn bounding_box(&self) -> Result<Option<Rect>>;
    async fn scroll_into_view(&self) -> Result<()>;
    async fn screenshot(&self) -> Result<Screenshot>;
    async fn select_option(&self, values: Vec<String>) -> Result<()>;
    async fn query_selector(&self, selector: &str) -> Result<Option<Arc<dyn ElementBackend>>>;
    async fn query_selector_all(&self, selector: &str) -> Result<Vec<Arc<dyn ElementBackend>>>;
    async fn parent(&self) -> Result<Option<Arc<dyn ElementBackend>>>;
    async fn evaluate(&self, script: &str) -> Result<Value>;
    async fn tag_name(&self) -> Result<String>;
    async fn input_value(&self) -> Result<String>;
}

#[async_trait]
pub(crate) trait FrameBackend: Send + Sync {
    async fn url(&self) -> Result<String>;
    async fn title(&self) -> Result<String>;
    async fn query_selector(&self, selector: &str) -> Result<Option<Arc<dyn ElementBackend>>>;
    async fn evaluate(&self, script: &str) -> Result<Value>;
    async fn wait_for_selector(&self, selector: &str) -> Result<Arc<dyn ElementBackend>>;
}

// ============================================================================
// Data types
// ============================================================================

/// A screenshot result.
#[derive(Debug, Clone)]
pub struct Screenshot {
    /// Raw image bytes (PNG format by default).
    pub data: Vec<u8>,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Image format.
    pub format: ScreenshotFormat,
}

impl Screenshot {
    /// Save the screenshot to a file.
    pub fn save(&self, path: &str) -> Result<()> {
        std::fs::write(path, &self.data)?;
        Ok(())
    }

    /// Get the screenshot as base64 encoded string.
    #[cfg(feature = "base64")]
    pub fn to_base64(&self) -> String {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(&self.data)
    }
}

/// Screenshot format options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScreenshotFormat {
    /// PNG format (default, lossless).
    #[default]
    Png,
    /// JPEG format (lossy, smaller file size).
    Jpeg,
}

/// Screenshot options.
#[derive(Debug, Clone, Default)]
pub struct ScreenshotOptions {
    /// Screenshot format.
    pub format: ScreenshotFormat,
    /// Quality for JPEG (0-100).
    pub quality: Option<u8>,
    /// Clip to a specific region.
    pub clip: Option<Rect>,
    /// Capture full page (not just viewport).
    pub full_page: bool,
    /// Hide specific elements.
    pub hide_selectors: Vec<String>,
}

impl ScreenshotOptions {
    /// Create new options with PNG format.
    pub fn png() -> Self {
        Self {
            format: ScreenshotFormat::Png,
            ..Default::default()
        }
    }

    /// Create new options with JPEG format.
    pub fn jpeg(quality: u8) -> Self {
        Self {
            format: ScreenshotFormat::Jpeg,
            quality: Some(quality.min(100)),
            ..Default::default()
        }
    }

    /// Capture the full page.
    pub fn full_page(mut self) -> Self {
        self.full_page = true;
        self
    }

    /// Clip to a specific region.
    pub fn clip(mut self, rect: Rect) -> Self {
        self.clip = Some(rect);
        self
    }

    /// Hide elements matching these selectors.
    pub fn hide(mut self, selectors: Vec<String>) -> Self {
        self.hide_selectors = selectors;
        self
    }
}

/// Rectangle geometry.
#[derive(Debug, Clone, Copy, Default)]
pub struct Rect {
    /// X coordinate.
    pub x: f64,
    /// Y coordinate.
    pub y: f64,
    /// Width.
    pub width: f64,
    /// Height.
    pub height: f64,
}

impl Rect {
    /// Create a new rectangle.
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

/// Cookie representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cookie {
    /// Cookie name.
    pub name: String,
    /// Cookie value.
    pub value: String,
    /// Cookie domain.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    /// Cookie path.
    #[serde(default = "default_path")]
    pub path: String,
    /// Expiration timestamp (Unix epoch seconds).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires: Option<i64>,
    /// Whether the cookie is HTTP-only.
    #[serde(default)]
    pub http_only: bool,
    /// Whether the cookie is secure.
    #[serde(default)]
    pub secure: bool,
    /// SameSite attribute.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub same_site: Option<SameSite>,
}

impl Cookie {
    /// Create a new cookie with just name and value.
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            domain: None,
            path: default_path(),
            expires: None,
            http_only: false,
            secure: false,
            same_site: None,
        }
    }

    /// Set the domain.
    pub fn with_domain(mut self, domain: impl Into<String>) -> Self {
        self.domain = Some(domain.into());
        self
    }

    /// Set the path.
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }
}

fn default_path() -> String {
    "/".to_string()
}

/// SameSite cookie attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum SameSite {
    /// Strict SameSite policy.
    Strict,
    /// Lax SameSite policy.
    Lax,
    /// None SameSite policy.
    None,
}

/// Page load state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LoadState {
    /// DOM is loaded.
    DomContentLoaded,
    /// All resources are loaded.
    Load,
    /// Network is idle (no connections for 500ms).
    NetworkIdle,
}

/// PDF generation options.
#[derive(Debug, Clone, Default)]
pub struct PdfOptions {
    /// Paper width in inches.
    pub width: Option<f64>,
    /// Paper height in inches.
    pub height: Option<f64>,
    /// Paper format (Letter, A4, etc.).
    pub format: Option<String>,
    /// Whether to print background graphics.
    pub print_background: bool,
    /// Page scale factor.
    pub scale: Option<f64>,
    /// Top margin in inches.
    pub margin_top: Option<f64>,
    /// Bottom margin in inches.
    pub margin_bottom: Option<f64>,
    /// Left margin in inches.
    pub margin_left: Option<f64>,
    /// Right margin in inches.
    pub margin_right: Option<f64>,
    /// Page ranges to print (e.g., "1-5, 8, 11-13").
    pub page_ranges: Option<String>,
    /// Whether to display header and footer.
    pub display_header_footer: bool,
    /// HTML template for the header.
    pub header_template: Option<String>,
    /// HTML template for the footer.
    pub footer_template: Option<String>,
    /// Whether to prefer page size defined by CSS.
    pub prefer_css_page_size: bool,
}

impl PdfOptions {
    /// Create default PDF options.
    pub fn new() -> Self {
        Self::default()
    }

    /// A4 format.
    pub fn a4() -> Self {
        Self {
            format: Some("A4".to_string()),
            ..Default::default()
        }
    }

    /// Letter format.
    pub fn letter() -> Self {
        Self {
            format: Some("Letter".to_string()),
            ..Default::default()
        }
    }

    /// Print with background.
    pub fn with_background(mut self) -> Self {
        self.print_background = true;
        self
    }

    /// Set scale factor.
    pub fn scale(mut self, scale: f64) -> Self {
        self.scale = Some(scale);
        self
    }
}
