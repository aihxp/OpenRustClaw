//! Browser automation capabilities for OpenRustClaw.
//!
//! This crate provides browser automation through two backends:
//! - **Playwright**: High-level browser automation with excellent cross-browser support
//! - **CDP (Chrome DevTools Protocol)**: Direct Chrome control for lower-level access
//!
//! # Example Usage
//!
//! ```rust,no_run
//! use openrustclaw_automation::{Browser, BrowserConfig};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Create browser with default configuration
//!     let config = BrowserConfig::default();
//!     let browser = Browser::new(config).await?;
//!
//!     // Navigate to a page
//!     let page = browser.navigate("https://example.com").await?;
//!
//!     // Take a screenshot
//!     let screenshot = page.screenshot().await?;
//!
//!     // Close the browser
//!     browser.close().await?;
//!     Ok(())
//! }
//! ```

pub mod browser;
pub mod cdp;
pub mod config;
pub mod error;
pub mod playwright;
pub mod tools;
pub mod vision;

// Re-exports for convenience
pub use browser::{Browser, Page};
pub use config::{AutomationConfig, BrowserConfig, BrowserType, Viewport};
pub use error::{AutomationError, Result};

#[cfg(feature = "playwright")]
pub use playwright::PlaywrightBackend;

#[cfg(feature = "cdp")]
pub use cdp::CdpBackend;

/// Version of the automation crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
