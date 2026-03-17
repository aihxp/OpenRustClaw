//! Visual understanding capabilities for browser automation.
//!
//! This module provides computer vision and OCR functionality
//! to enable visual understanding of web pages.
//!
//! # Features
//! - OCR (Optical Character Recognition) on screenshots
//! - Visual element detection
//! - Image comparison and diffing
//! - Visual regression testing

pub mod ocr;

use image::GenericImageView;

use crate::browser::Screenshot;
use crate::error::Result;

/// Visual understanding capabilities.
#[derive(Debug, Clone)]
pub struct VisionCapabilities {
    ocr_enabled: bool,
}

impl VisionCapabilities {
    /// Create new vision capabilities.
    pub fn new() -> Self {
        Self {
            ocr_enabled: cfg!(feature = "ocr"),
        }
    }

    /// Check if OCR is enabled.
    pub fn ocr_enabled(&self) -> bool {
        self.ocr_enabled
    }

    /// Extract text from a screenshot using OCR.
    #[cfg(feature = "ocr")]
    pub async fn extract_text(&self, screenshot: &Screenshot) -> Result<String> {
        ocr::extract_text(&screenshot.data).await
    }

    /// Extract text from a screenshot (stub when OCR is disabled).
    #[cfg(not(feature = "ocr"))]
    pub async fn extract_text(&self, _screenshot: &Screenshot) -> Result<String> {
        Err(crate::error::AutomationError::Other(
            "OCR feature is not enabled. Enable the 'ocr' feature to use this functionality."
                .to_string(),
        ))
    }

    /// Find text on the page by OCR.
    #[cfg(feature = "ocr")]
    pub async fn find_text(
        &self,
        screenshot: &Screenshot,
        text: &str,
    ) -> Result<Vec<TextRegion>> {
        ocr::find_text(&screenshot.data, text).await
    }

    /// Find text on the page (stub when OCR is disabled).
    #[cfg(not(feature = "ocr"))]
    pub async fn find_text(
        &self,
        _screenshot: &Screenshot,
        _text: &str,
    ) -> Result<Vec<TextRegion>> {
        Err(crate::error::AutomationError::Other(
            "OCR feature is not enabled".to_string(),
        ))
    }

    /// Compare two screenshots and return differences.
    pub async fn compare_screenshots(
        &self,
        baseline: &Screenshot,
        current: &Screenshot,
    ) -> Result<ScreenshotDiff> {
        // Simple pixel-by-pixel comparison
        if baseline.width != current.width || baseline.height != current.height {
            return Ok(ScreenshotDiff {
                identical: false,
                diff_percentage: 100.0,
                diff_regions: vec![],
            });
        }

        // Decode images
        let baseline_img = image::load_from_memory(&baseline.data)
            .map_err(|e| crate::error::AutomationError::Other(format!("Failed to decode baseline image: {}", e)))?;
        let current_img = image::load_from_memory(&current.data)
            .map_err(|e| crate::error::AutomationError::Other(format!("Failed to decode current image: {}", e)))?;

        let mut diff_pixels = 0u64;
        let total_pixels = (baseline.width * baseline.height) as u64;

        for y in 0..baseline.height {
            for x in 0..baseline.width {
                let baseline_pixel = baseline_img.get_pixel(x, y);
                let current_pixel = current_img.get_pixel(x, y);

                if baseline_pixel != current_pixel {
                    diff_pixels += 1;
                }
            }
        }

        let diff_percentage = (diff_pixels as f64 / total_pixels as f64) * 100.0;

        Ok(ScreenshotDiff {
            identical: diff_pixels == 0,
            diff_percentage,
            diff_regions: vec![], // Could implement region detection
        })
    }
}

impl Default for VisionCapabilities {
    fn default() -> Self {
        Self::new()
    }
}

/// A region of detected text.
#[derive(Debug, Clone)]
pub struct TextRegion {
    /// The detected text.
    pub text: String,
    /// Bounding box (x, y, width, height).
    pub bbox: (u32, u32, u32, u32),
    /// Confidence score (0.0 - 1.0).
    pub confidence: f32,
}

/// Screenshot comparison result.
#[derive(Debug, Clone)]
pub struct ScreenshotDiff {
    /// Whether the screenshots are identical.
    pub identical: bool,
    /// Percentage of different pixels.
    pub diff_percentage: f64,
    /// Regions that differ.
    pub diff_regions: Vec<DiffRegion>,
}

impl ScreenshotDiff {
    /// Check if the difference is significant (> 1%).
    pub fn is_significant(&self) -> bool {
        self.diff_percentage > 1.0
    }
}

/// A region of difference between two screenshots.
#[derive(Debug, Clone)]
pub struct DiffRegion {
    /// Bounding box of the difference.
    pub bbox: (u32, u32, u32, u32),
    /// Magnitude of difference in this region.
    pub magnitude: f64,
}

/// Visual element types that can be detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VisualElementType {
    /// Button element.
    Button,
    /// Text input.
    TextInput,
    /// Checkbox.
    Checkbox,
    /// Radio button.
    RadioButton,
    /// Dropdown/select.
    Dropdown,
    /// Link.
    Link,
    /// Image.
    Image,
    /// Text block.
    Text,
    /// Heading.
    Heading,
    /// Navigation menu.
    Navigation,
    /// Search box.
    Search,
    /// Logo.
    Logo,
    /// Modal/dialog.
    Modal,
    /// Notification/toast.
    Notification,
}

/// A detected visual element.
#[derive(Debug, Clone)]
pub struct VisualElement {
    /// Type of element.
    pub element_type: VisualElementType,
    /// Bounding box.
    pub bbox: (u32, u32, u32, u32),
    /// Confidence score.
    pub confidence: f32,
    /// Text content (if any).
    pub text: Option<String>,
}

/// Visual matcher for finding elements by appearance.
pub struct VisualMatcher {
    vision: VisionCapabilities,
}

impl VisualMatcher {
    /// Create a new visual matcher.
    pub fn new() -> Self {
        Self {
            vision: VisionCapabilities::new(),
        }
    }

    /// Find an element by its visual appearance.
    pub async fn find_by_appearance(
        &self,
        _screenshot: &Screenshot,
        _element_type: VisualElementType,
    ) -> Result<Vec<VisualElement>> {
        // This would use computer vision models in a real implementation
        Ok(vec![])
    }

    /// Find a button by its label.
    pub async fn find_button(&self, screenshot: &Screenshot, label: &str) -> Result<Option<VisualElement>> {
        if !self.vision.ocr_enabled() {
            return Ok(None);
        }

        #[cfg(feature = "ocr")]
        {
            let regions = self.vision.find_text(screenshot, label).await?;
            Ok(regions.into_iter().next().map(|r| VisualElement {
                element_type: VisualElementType::Button,
                bbox: r.bbox,
                confidence: r.confidence,
                text: Some(r.text),
            }))
        }

        #[cfg(not(feature = "ocr"))]
        {
            let _ = screenshot;
            let _ = label;
            Ok(None)
        }
    }
}

impl Default for VisualMatcher {
    fn default() -> Self {
        Self::new()
    }
}
