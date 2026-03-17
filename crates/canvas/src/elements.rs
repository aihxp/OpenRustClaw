//! Canvas UI elements - building blocks for the visual workspace

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A UI element that can be rendered on the canvas
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CanvasElement {
    /// Text content with styling
    Text {
        id: Uuid,
        content: String,
        style: TextStyle,
    },
    /// Image display
    Image {
        id: Uuid,
        url: String,
        alt: String,
        size: ImageSize,
    },
    /// Data visualization chart
    Chart {
        id: Uuid,
        chart_type: ChartType,
        data: ChartData,
        title: String,
    },
    /// Interactive form
    Form {
        id: Uuid,
        fields: Vec<FormField>,
        submit_label: String,
    },
    /// Clickable button
    Button {
        id: Uuid,
        label: String,
        action: ButtonAction,
        style: ButtonStyle,
    },
    /// Code block with optional execution
    Code {
        id: Uuid,
        language: String,
        code: String,
        runnable: bool,
    },
    /// Markdown content
    Markdown { id: Uuid, content: String },
    /// Container for layout management
    Container {
        id: Uuid,
        layout: Layout,
        children: Vec<CanvasElement>,
    },
}

impl CanvasElement {
    /// Get the unique identifier of this element
    pub fn id(&self) -> Uuid {
        match self {
            CanvasElement::Text { id, .. } => *id,
            CanvasElement::Image { id, .. } => *id,
            CanvasElement::Chart { id, .. } => *id,
            CanvasElement::Form { id, .. } => *id,
            CanvasElement::Button { id, .. } => *id,
            CanvasElement::Code { id, .. } => *id,
            CanvasElement::Markdown { id, .. } => *id,
            CanvasElement::Container { id, .. } => *id,
        }
    }
}

/// Text styling options
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TextStyle {
    pub font_size: u32,
    pub color: String,
    pub bold: bool,
    pub italic: bool,
}

impl TextStyle {
    /// Create a new text style with default values
    pub fn new() -> Self {
        Self {
            font_size: 14,
            color: "#000000".to_string(),
            bold: false,
            italic: false,
        }
    }

    /// Set the font size
    pub fn font_size(mut self, size: u32) -> Self {
        self.font_size = size;
        self
    }

    /// Set the text color (hex format)
    pub fn color(mut self, color: impl Into<String>) -> Self {
        self.color = color.into();
        self
    }

    /// Set bold formatting
    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    /// Set italic formatting
    pub fn italic(mut self) -> Self {
        self.italic = true;
        self
    }
}

/// Image sizing options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSize {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fit: ImageFit,
}

impl Default for ImageSize {
    fn default() -> Self {
        Self {
            width: None,
            height: None,
            fit: ImageFit::Contain,
        }
    }
}

/// Image fitting behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageFit {
    Contain,
    Cover,
    Fill,
    None,
}

/// Chart types for data visualization
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChartType {
    Line,
    Bar,
    Pie,
    Scatter,
}

/// Chart data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartData {
    pub labels: Vec<String>,
    pub datasets: Vec<Dataset>,
}

/// Dataset for charts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dataset {
    pub label: String,
    pub data: Vec<f64>,
    pub color: Option<String>,
}

/// Form field definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormField {
    pub name: String,
    pub label: String,
    pub field_type: FieldType,
    pub required: bool,
    pub options: Option<Vec<String>>,
    pub placeholder: Option<String>,
}

/// Field types for forms
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FieldType {
    Text,
    Number,
    Email,
    Select,
    Checkbox,
    Textarea,
}

/// Button action types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ButtonAction {
    Click,
    Submit { form_id: Uuid },
    Navigate { url: String },
    Custom { action_id: String },
}

/// Button styling options
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ButtonStyle {
    pub variant: ButtonVariant,
    pub size: ButtonSize,
    pub disabled: bool,
}

/// Button visual variants
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
    Success,
    Danger,
    Ghost,
}

/// Button sizes
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ButtonSize {
    #[default]
    Medium,
    Small,
    Large,
}

/// Layout options for containers
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Layout {
    Vertical { gap: u32 },
    Horizontal { gap: u32 },
    Grid { columns: u32, gap: u32 },
}

impl Default for Layout {
    fn default() -> Self {
        Layout::Vertical { gap: 16 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_style_builder() {
        let style = TextStyle::new()
            .font_size(16)
            .color("#333333")
            .bold()
            .italic();

        assert_eq!(style.font_size, 16);
        assert_eq!(style.color, "#333333");
        assert!(style.bold);
        assert!(style.italic);
    }

    #[test]
    fn test_canvas_element_id() {
        let id = Uuid::new_v4();
        let element = CanvasElement::Text {
            id,
            content: "Hello".to_string(),
            style: TextStyle::new(),
        };

        assert_eq!(element.id(), id);
    }

    #[test]
    fn test_serialize_text_element() {
        let element = CanvasElement::Text {
            id: Uuid::new_v4(),
            content: "Hello World".to_string(),
            style: TextStyle::new(),
        };

        let json = serde_json::to_string(&element).unwrap();
        assert!(json.contains("\"type\":\"text\""));
        assert!(json.contains("Hello World"));
    }
}
