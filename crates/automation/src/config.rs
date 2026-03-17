//! Configuration types for browser automation.

use serde::{Deserialize, Serialize};

/// Browser automation configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationConfig {
    /// Default browser to use.
    #[serde(default)]
    pub browser: BrowserType,

    /// Whether to run browsers in headless mode.
    #[serde(default = "default_headless")]
    pub headless: bool,

    /// Default viewport size.
    #[serde(default)]
    pub viewport: Viewport,

    /// Custom user agent (None for default).
    #[serde(default)]
    pub user_agent: Option<String>,

    /// Default navigation timeout in milliseconds.
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,

    /// Whether to accept insecure certificates.
    #[serde(default)]
    pub ignore_https_errors: bool,

    /// Additional browser arguments.
    #[serde(default)]
    pub args: Vec<String>,

    /// Download directory for files.
    #[serde(default)]
    pub download_path: Option<String>,

    /// Locale setting.
    #[serde(default = "default_locale")]
    pub locale: String,

    /// Timezone setting.
    #[serde(default = "default_timezone")]
    pub timezone: String,

    /// Color scheme preference.
    #[serde(default)]
    pub color_scheme: ColorScheme,
}

impl Default for AutomationConfig {
    fn default() -> Self {
        Self {
            browser: BrowserType::default(),
            headless: default_headless(),
            viewport: Viewport::default(),
            user_agent: None,
            timeout_ms: default_timeout(),
            ignore_https_errors: false,
            args: Vec::new(),
            download_path: None,
            locale: default_locale(),
            timezone: default_timezone(),
            color_scheme: ColorScheme::default(),
        }
    }
}

/// Browser type selection.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserType {
    /// Chromium/Chrome.
    #[default]
    Chromium,
    /// Firefox.
    Firefox,
    /// WebKit/Safari.
    Webkit,
    /// Microsoft Edge.
    Edge,
}

impl std::fmt::Display for BrowserType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Chromium => write!(f, "chromium"),
            Self::Firefox => write!(f, "firefox"),
            Self::Webkit => write!(f, "webkit"),
            Self::Edge => write!(f, "edge"),
        }
    }
}

/// Viewport configuration.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Viewport {
    /// Viewport width in pixels.
    #[serde(default = "default_width")]
    pub width: u32,
    /// Viewport height in pixels.
    #[serde(default = "default_height")]
    pub height: u32,
    /// Device scale factor (DPR).
    #[serde(default = "default_device_scale_factor")]
    pub device_scale_factor: f64,
    /// Whether the viewport is mobile.
    #[serde(default)]
    pub is_mobile: bool,
    /// Whether the viewport has touch enabled.
    #[serde(default)]
    pub has_touch: bool,
    /// Whether the viewport is in landscape mode.
    #[serde(default)]
    pub is_landscape: bool,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            width: default_width(),
            height: default_height(),
            device_scale_factor: default_device_scale_factor(),
            is_mobile: false,
            has_touch: false,
            is_landscape: false,
        }
    }
}

impl Viewport {
    /// Common viewport presets.
    pub fn desktop() -> Self {
        Self::default()
    }

    /// Mobile viewport (iPhone 12/13/14).
    pub fn mobile() -> Self {
        Self {
            width: 390,
            height: 844,
            device_scale_factor: 3.0,
            is_mobile: true,
            has_touch: true,
            is_landscape: false,
        }
    }

    /// Tablet viewport (iPad).
    pub fn tablet() -> Self {
        Self {
            width: 810,
            height: 1080,
            device_scale_factor: 2.0,
            is_mobile: true,
            has_touch: true,
            is_landscape: false,
        }
    }
}

/// Color scheme preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColorScheme {
    /// Light color scheme.
    #[default]
    Light,
    /// Dark color scheme.
    Dark,
    /// Follow system preference.
    NoPreference,
}

/// Browser-specific configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BrowserConfig {
    /// The automation config.
    #[serde(flatten)]
    pub automation: AutomationConfig,

    /// Browser-specific arguments.
    #[serde(default)]
    pub browser_args: Vec<String>,

    /// Path to browser executable.
    #[serde(default)]
    pub executable_path: Option<String>,

    /// Proxy configuration.
    #[serde(default)]
    pub proxy: Option<ProxyConfig>,

    /// Geolocation override.
    #[serde(default)]
    pub geolocation: Option<Geolocation>,

    /// Permission overrides.
    #[serde(default)]
    pub permissions: Vec<Permission>,

    /// Record video of sessions.
    #[serde(default)]
    pub record_video: Option<VideoConfig>,

    /// Tracing configuration for performance analysis.
    #[serde(default)]
    pub tracing: Option<TracingConfig>,
}

impl BrowserConfig {
    /// Create a new config with the given browser type.
    pub fn with_browser(browser: BrowserType) -> Self {
        Self {
            automation: AutomationConfig {
                browser,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// Enable headless mode.
    pub fn headless(mut self) -> Self {
        self.automation.headless = true;
        self
    }

    /// Disable headless mode.
    pub fn headed(mut self) -> Self {
        self.automation.headless = false;
        self
    }

    /// Set the viewport.
    pub fn viewport(mut self, viewport: Viewport) -> Self {
        self.automation.viewport = viewport;
        self
    }

    /// Set custom user agent.
    pub fn user_agent(mut self, ua: impl Into<String>) -> Self {
        self.automation.user_agent = Some(ua.into());
        self
    }

    /// Set timeout.
    pub fn timeout_ms(mut self, ms: u64) -> Self {
        self.automation.timeout_ms = ms;
        self
    }
}

/// Proxy configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// Proxy server URL.
    pub server: String,
    /// Proxy bypass patterns.
    #[serde(default)]
    pub bypass: Vec<String>,
    /// Proxy username.
    #[serde(default)]
    pub username: Option<String>,
    /// Proxy password.
    #[serde(default)]
    pub password: Option<String>,
}

/// Geolocation configuration.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Geolocation {
    /// Latitude.
    pub latitude: f64,
    /// Longitude.
    pub longitude: f64,
    /// Accuracy in meters.
    #[serde(default = "default_accuracy")]
    pub accuracy: f64,
}

impl Geolocation {
    /// Create a new geolocation.
    pub fn new(latitude: f64, longitude: f64) -> Self {
        Self {
            latitude,
            longitude,
            accuracy: default_accuracy(),
        }
    }
}

/// Browser permissions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    /// Geolocation access.
    Geolocation,
    /// MIDI device access.
    Midi,
    /// Notifications.
    Notifications,
    /// Camera access.
    Camera,
    /// Microphone access.
    Microphone,
    /// Background sync.
    BackgroundSync,
    /// Ambient light sensor.
    AmbientLightSensor,
    /// Accelerometer.
    Accelerometer,
    /// Gyroscope.
    Gyroscope,
    /// Magnetometer.
    Magnetometer,
    /// Accessibility events.
    AccessibilityEvents,
    /// Clipboard read.
    ClipboardRead,
    /// Clipboard write.
    ClipboardWrite,
    /// Payment handler.
    PaymentHandler,
    /// Persistent storage.
    PersistentStorage,
}

/// Video recording configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoConfig {
    /// Directory to save videos.
    pub dir: String,
    /// Video size (defaults to viewport size).
    #[serde(default)]
    pub size: Option<(u32, u32)>,
}

/// Tracing configuration for performance analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracingConfig {
    /// Path to save trace file.
    pub path: String,
    /// Capture screenshots.
    #[serde(default)]
    pub screenshots: bool,
    /// Capture network activity.
    #[serde(default = "default_true")]
    pub network: bool,
}

// Default value functions
fn default_headless() -> bool {
    true
}

fn default_timeout() -> u64 {
    30000 // 30 seconds
}

fn default_locale() -> String {
    "en-US".to_string()
}

fn default_timezone() -> String {
    "UTC".to_string()
}

fn default_width() -> u32 {
    1920
}

fn default_height() -> u32 {
    1080
}

fn default_device_scale_factor() -> f64 {
    1.0
}

fn default_accuracy() -> f64 {
    0.0
}

fn default_true() -> bool {
    true
}
