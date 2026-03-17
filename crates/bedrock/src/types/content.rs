//! Content block types for Bedrock messages.

use serde::{Deserialize, Serialize};

/// A content block in a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Text content.
    Text {
        /// The text content.
        text: String,
    },
    /// Image content.
    Image {
        /// The image data.
        #[serde(rename = "source")]
        image: ImageBlock,
    },
    /// Video content.
    Video {
        /// The video data.
        #[serde(rename = "source")]
        video: VideoBlock,
    },
    /// Tool use request.
    ToolUse {
        /// Tool use block.
        #[serde(rename = "toolUse")]
        tool_use: ToolUseBlock,
    },
    /// Tool result.
    ToolResult {
        /// Tool result block.
        #[serde(rename = "toolResult")]
        tool_result: ToolResultBlock,
    },
    /// Guardrail content.
    GuardContent {
        /// Guardrail assessment.
        #[serde(rename = "guardContent")]
        guard_content: GuardrailAssessment,
    },
}

impl ContentBlock {
    /// Create a text content block.
    pub fn text(content: impl Into<String>) -> Self {
        ContentBlock::Text {
            text: content.into(),
        }
    }

    /// Create an image content block from S3.
    pub fn image_from_s3(bucket: impl Into<String>, key: impl Into<String>) -> Self {
        ContentBlock::Image {
            image: ImageBlock {
                format: ImageFormat::Jpeg, // Format doesn't matter for S3
                source: ImageSource::S3Location(S3Location {
                    uri: None,
                    bucket: Some(bucket.into()),
                    key: Some(key.into()),
                }),
            },
        }
    }

    /// Create an image content block from base64 bytes.
    pub fn image_from_bytes(
        format: ImageFormat,
        bytes: impl Into<String>,
    ) -> Self {
        ContentBlock::Image {
            image: ImageBlock {
                format,
                source: ImageSource::Bytes(bytes.into()),
            },
        }
    }

    /// Create a tool use block.
    pub fn tool_use(
        tool_use_id: impl Into<String>,
        name: impl Into<String>,
        input: serde_json::Value,
    ) -> Self {
        ContentBlock::ToolUse {
            tool_use: ToolUseBlock {
                tool_use_id: tool_use_id.into(),
                name: name.into(),
                input,
            },
        }
    }

    /// Create a tool result block.
    pub fn tool_result(
        tool_use_id: impl Into<String>,
        content: Vec<ToolResultContent>,
        is_error: bool,
    ) -> Self {
        ContentBlock::ToolResult {
            tool_result: ToolResultBlock {
                tool_use_id: tool_use_id.into(),
                content,
                is_error: Some(is_error),
            },
        }
    }

    /// Get the text content if this is a text block.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            ContentBlock::Text { text } => Some(text),
            _ => None,
        }
    }

    /// Check if this is a tool use block.
    pub fn is_tool_use(&self) -> bool {
        matches!(self, ContentBlock::ToolUse { .. })
    }

    /// Check if this is a tool result block.
    pub fn is_tool_result(&self) -> bool {
        matches!(self, ContentBlock::ToolResult { .. })
    }
}

/// Image format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageFormat {
    /// JPEG format.
    Jpeg,
    /// PNG format.
    Png,
    /// GIF format.
    Gif,
    /// WebP format.
    Webp,
}

impl ImageFormat {
    /// Get the MIME type for this format.
    pub fn mime_type(&self) -> &'static str {
        match self {
            ImageFormat::Jpeg => "image/jpeg",
            ImageFormat::Png => "image/png",
            ImageFormat::Gif => "image/gif",
            ImageFormat::Webp => "image/webp",
        }
    }
}

/// An image block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageBlock {
    /// The image format.
    pub format: ImageFormat,
    /// The image source.
    pub source: ImageSource,
}

/// Image source.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageSource {
    /// Base64-encoded image bytes.
    Bytes(String),
    /// S3 location.
    S3Location(S3Location),
}

/// Video format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VideoFormat {
    /// MP4 format.
    Mp4,
    /// QuickTime format.
    Mov,
    /// MKV format.
    Mkv,
    /// WebM format.
    Webm,
    /// FLV format.
    Flv,
    /// MPEG format.
    Mpeg,
    /// MPG format.
    Mpg,
    /// WMV format.
    Wmv,
    /// 3GP format.
    ThreeGp,
}

impl VideoFormat {
    /// Get the MIME type for this format.
    pub fn mime_type(&self) -> &'static str {
        match self {
            VideoFormat::Mp4 => "video/mp4",
            VideoFormat::Mov => "video/quicktime",
            VideoFormat::Mkv => "video/x-matroska",
            VideoFormat::Webm => "video/webm",
            VideoFormat::Flv => "video/x-flv",
            VideoFormat::Mpeg => "video/mpeg",
            VideoFormat::Mpg => "video/mpeg",
            VideoFormat::Wmv => "video/x-ms-wmv",
            VideoFormat::ThreeGp => "video/3gpp",
        }
    }
}

/// A video block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoBlock {
    /// The video format.
    pub format: VideoFormat,
    /// The video source.
    pub source: VideoSource,
}

/// Video source.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VideoSource {
    /// Base64-encoded video bytes.
    Bytes(String),
    /// S3 location.
    S3Location(S3Location),
}

/// S3 location.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct S3Location {
    /// The S3 URI (e.g., s3://bucket/key).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    /// The S3 bucket.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bucket: Option<String>,
    /// The S3 key.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
}

impl S3Location {
    /// Create from URI.
    pub fn from_uri(uri: impl Into<String>) -> Self {
        Self {
            uri: Some(uri.into()),
            bucket: None,
            key: None,
        }
    }

    /// Create from bucket and key.
    pub fn from_bucket_and_key(bucket: impl Into<String>, key: impl Into<String>) -> Self {
        Self {
            uri: None,
            bucket: Some(bucket.into()),
            key: Some(key.into()),
        }
    }
}

/// Tool use block.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolUseBlock {
    /// The tool use ID.
    pub tool_use_id: String,
    /// The tool name.
    pub name: String,
    /// The tool input (JSON object).
    pub input: serde_json::Value,
}

/// Tool result block.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolResultBlock {
    /// The tool use ID this is a result for.
    pub tool_use_id: String,
    /// The result content.
    pub content: Vec<ToolResultContent>,
    /// Whether this result represents an error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

/// Tool result content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolResultContent {
    /// Text content.
    Text {
        /// The text content.
        text: String,
    },
    /// Image content.
    Image {
        /// The image.
        image: ImageBlock,
    },
    /// Video content.
    Video {
        /// The video.
        video: VideoBlock,
    },
    /// JSON content.
    Json {
        /// The JSON content.
        json: serde_json::Value,
    },
}

impl ToolResultContent {
    /// Create text content.
    pub fn text(content: impl Into<String>) -> Self {
        ToolResultContent::Text {
            text: content.into(),
        }
    }

    /// Create JSON content.
    pub fn json(value: impl Into<serde_json::Value>) -> Self {
        ToolResultContent::Json { json: value.into() }
    }
}

/// Delta in a streaming content block.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlockDelta {
    /// Text delta.
    Text {
        /// The text delta.
        text: String,
    },
    /// Partial tool input JSON.
    ToolUse {
        /// Tool use delta.
        #[serde(rename = "toolUse")]
        tool_use: ToolUseDelta,
    },
}

impl ContentBlockDelta {
    /// Get the text if this is a text delta.
    pub fn text(&self) -> Option<&str> {
        match self {
            ContentBlockDelta::Text { text } => Some(text),
            _ => None,
        }
    }
}

/// Tool use delta for streaming.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolUseDelta {
    /// The tool use ID.
    pub tool_use_id: String,
    /// The name of the tool.
    pub name: String,
    /// Partial JSON input.
    pub input: String,
}

// Guardrail types

/// Guardrail content filter.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuardrailContentFilter {
    /// The confidence level.
    pub confidence: String,
    /// The type of content.
    #[serde(rename = "type")]
    pub filter_type: String,
    /// The action taken.
    pub action: String,
}

/// Guardrail PII entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuardrailPiiEntity {
    /// The PII type.
    #[serde(rename = "type")]
    pub entity_type: String,
    /// Whether a match was found.
    pub match_: bool,
    /// The action taken.
    pub action: String,
}

/// Guardrail regex.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuardrailRegex {
    /// The regex name.
    pub name: String,
    /// The regex pattern.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    /// The match found.
    #[serde(rename = "match", skip_serializing_if = "Option::is_none")]
    pub match_: Option<String>,
    /// The action taken.
    pub action: String,
}

/// Guardrail custom word.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuardrailCustomWord {
    /// The match found.
    #[serde(rename = "match")]
    pub match_: String,
    /// The action taken.
    pub action: String,
}

/// Guardrail managed word type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GuardrailManagedWordType {
    /// Profanity.
    Profanity,
}

/// Guardrail managed word.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuardrailManagedWord {
    /// The match found.
    #[serde(rename = "match")]
    pub match_: String,
    /// The type of managed word.
    #[serde(rename = "type")]
    pub word_type: GuardrailManagedWordType,
    /// The action taken.
    pub action: String,
}

/// Guardrail topic.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuardrailTopic {
    /// The topic name.
    pub name: String,
    /// The topic type.
    #[serde(rename = "type")]
    pub topic_type: String,
    /// The action taken.
    pub action: String,
}

/// Guardrail assessment.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GuardrailAssessment {
    /// Topic policy assessment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic_policy: Option<GuardrailTopicPolicyAction>,
    /// Content policy assessment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_policy: Option<GuardrailContentPolicyAction>,
    /// Word policy assessment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub word_policy: Option<GuardrailWordPolicyAction>,
    /// Sensitive information policy assessment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sensitive_information_policy: Option<GuardrailSensitiveInformationPolicyAction>,
}

/// Guardrail topic policy action.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuardrailTopicPolicyAction {
    /// The topics.
    pub topics: Vec<GuardrailTopic>,
}

/// Guardrail content policy action.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuardrailContentPolicyAction {
    /// The filters.
    pub filters: Vec<GuardrailContentFilter>,
}

/// Guardrail word policy action.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuardrailWordPolicyAction {
    /// Custom words.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_words: Option<Vec<GuardrailCustomWord>>,
    /// Managed words.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub managed_word_lists: Option<Vec<GuardrailManagedWord>>,
}

/// Guardrail sensitive information policy action.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuardrailSensitiveInformationPolicyAction {
    /// PII entities.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pii_entities: Option<Vec<GuardrailPiiEntity>>,
    /// Regex patterns.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub regexes: Option<Vec<GuardrailRegex>>,
}

/// Guardrail content policy config.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GuardrailContentPolicyConfig {
    /// Filters configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters_config: Option<Vec<GuardrailContentFilterConfig>>,
}

/// Guardrail content filter config.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuardrailContentFilterConfig {
    /// The confidence threshold.
    pub confidence_threshold: String,
    /// The content filter type.
    #[serde(rename = "type")]
    pub filter_type: String,
    /// The input strength.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_strength: Option<String>,
    /// The output strength.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_strength: Option<String>,
}

/// Guardrail sensitive information policy config.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GuardrailSensitiveInformationPolicyConfig {
    /// PII entities configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pii_entities_config: Option<Vec<GuardrailPiiEntityConfig>>,
    /// Regex configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub regexes_config: Option<Vec<GuardrailRegexConfig>>,
}

/// Guardrail PII entity config.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuardrailPiiEntityConfig {
    /// The action.
    pub action: String,
    /// The PII type.
    #[serde(rename = "type")]
    pub entity_type: String,
}

/// Guardrail regex config.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuardrailRegexConfig {
    /// The action.
    pub action: String,
    /// The regex pattern.
    pub pattern: String,
    /// The regex name.
    pub name: String,
    /// The description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Guardrail topic policy config.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GuardrailTopicPolicyConfig {
    /// Topics configuration.
    pub topics_config: Vec<GuardrailTopicConfig>,
}

/// Guardrail topic config.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuardrailTopicConfig {
    /// The topic name.
    pub name: String,
    /// The topic definition.
    pub definition: String,
    /// Examples of the topic.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub examples: Option<Vec<String>>,
    /// The action.
    pub action: String,
    /// The topic type.
    #[serde(rename = "type")]
    pub topic_type: String,
}

/// Guardrail word policy config.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GuardrailWordPolicyConfig {
    /// Words configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub words_config: Option<Vec<GuardrailWordConfig>>,
    /// Managed word lists configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub managed_word_lists_config: Option<Vec<GuardrailManagedWordConfig>>,
}

/// Guardrail word config.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuardrailWordConfig {
    /// The word or phrase.
    pub text: String,
}

/// Guardrail managed word config.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuardrailManagedWordConfig {
    /// The type.
    #[serde(rename = "type")]
    pub word_type: String,
}

/// Guardrail word action.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuardrailWordAction {
    /// The word or phrase.
    pub text: String,
    /// The action.
    pub action: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_block_text() {
        let block = ContentBlock::text("Hello, world!");
        assert_eq!(block.as_text(), Some("Hello, world!"));
        assert!(!block.is_tool_use());
        assert!(!block.is_tool_result());
    }

    #[test]
    fn test_content_block_tool_use() {
        let block = ContentBlock::tool_use(
            "tool_123",
            "calculator",
            serde_json::json!({"expression": "2 + 2"}),
        );
        assert!(block.is_tool_use());
        assert!(block.as_text().is_none());
    }

    #[test]
    fn test_image_format() {
        assert_eq!(ImageFormat::Jpeg.mime_type(), "image/jpeg");
        assert_eq!(ImageFormat::Png.mime_type(), "image/png");
    }

    #[test]
    fn test_video_format() {
        assert_eq!(VideoFormat::Mp4.mime_type(), "video/mp4");
        assert_eq!(VideoFormat::Webm.mime_type(), "video/webm");
    }

    #[test]
    fn test_s3_location() {
        let loc = S3Location::from_uri("s3://mybucket/mykey");
        assert_eq!(loc.uri, Some("s3://mybucket/mykey".to_string()));

        let loc = S3Location::from_bucket_and_key("mybucket", "mykey");
        assert_eq!(loc.bucket, Some("mybucket".to_string()));
        assert_eq!(loc.key, Some("mykey".to_string()));
    }

    #[test]
    fn test_tool_result_content() {
        let text = ToolResultContent::text("Result text");
        match text {
            ToolResultContent::Text { text } => assert_eq!(text, "Result text"),
            _ => panic!("Expected text content"),
        }

        let json = ToolResultContent::json(serde_json::json!({"value": 42}));
        match json {
            ToolResultContent::Json { json } => assert_eq!(json["value"], 42),
            _ => panic!("Expected JSON content"),
        }
    }

    #[test]
    fn test_content_block_delta() {
        let delta = ContentBlockDelta::Text {
            text: "Hello".to_string(),
        };
        assert_eq!(delta.text(), Some("Hello"));
    }
}
