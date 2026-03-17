//! Gemini API types

use serde::{Deserialize, Serialize};

/// Gemini model variants
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GeminiModel {
    /// Gemini 1.5 Pro
    Gemini15Pro,
    /// Gemini 1.5 Flash
    Gemini15Flash,
    /// Gemini 1.0 Pro
    Gemini10Pro,
    /// Gemini 1.0 Pro Vision
    Gemini10ProVision,
    /// Gemini 1.0 Ultra
    Gemini10Ultra,
    /// Embedding model
    Embedding004,
    /// Custom model
    Custom(String),
}

impl GeminiModel {
    pub fn as_str(&self) -> &str {
        match self {
            GeminiModel::Gemini15Pro => "gemini-1.5-pro",
            GeminiModel::Gemini15Flash => "gemini-1.5-flash",
            GeminiModel::Gemini10Pro => "gemini-1.0-pro",
            GeminiModel::Gemini10ProVision => "gemini-1.0-pro-vision",
            GeminiModel::Gemini10Ultra => "gemini-1.0-ultra",
            GeminiModel::Embedding004 => "embedding-004",
            GeminiModel::Custom(s) => s.as_str(),
        }
    }
}

impl Default for GeminiModel {
    fn default() -> Self {
        GeminiModel::Gemini15Pro
    }
}



/// Request to generate content
#[derive(Debug, Clone, Serialize)]
pub struct GenerateContentRequest {
    pub contents: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_config: Option<ToolConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_settings: Option<Vec<SafetySetting>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GenerationConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<Content>,
}

/// Content structure (message)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    pub role: String,  // "user", "model", "system"
    pub parts: Vec<Part>,
}

impl Content {
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            parts: vec![Part::text(text)],
        }
    }
    
    pub fn model(text: impl Into<String>) -> Self {
        Self {
            role: "model".to_string(),
            parts: vec![Part::text(text)],
        }
    }
    
    pub fn system(text: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            parts: vec![Part::text(text)],
        }
    }
    
    pub fn with_image(self, mime_type: &str, data: Vec<u8>) -> Self {
        let mut parts = self.parts;
        parts.push(Part::inline_data(mime_type, data));
        Self { parts, ..self }
    }
}

/// Content part (text or inline data)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Part {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "inlineData")]
    InlineData { inline_data: Blob },
    #[serde(rename = "functionCall")]
    FunctionCall { function_call: FunctionCall },
    #[serde(rename = "functionResponse")]
    FunctionResponse { function_response: FunctionResponse },
}

impl Part {
    pub fn text(text: impl Into<String>) -> Self {
        Part::Text { text: text.into() }
    }
    
    pub fn inline_data(mime_type: impl Into<String>, data: Vec<u8>) -> Self {
        use base64::Engine;
        Part::InlineData {
            inline_data: Blob {
                mime_type: mime_type.into(),
                data: base64::engine::general_purpose::STANDARD.encode(data),
            },
        }
    }
}

/// Blob data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blob {
    pub mime_type: String,
    pub data: String,  // base64 encoded
}

/// Generate content response
#[derive(Debug, Clone, Deserialize)]
pub struct GenerateContentResponse {
    pub candidates: Vec<Candidate>,
    #[serde(rename = "usageMetadata")]
    pub usage_metadata: Option<UsageMetadata>,
    #[serde(rename = "promptFeedback")]
    pub prompt_feedback: Option<PromptFeedback>,
}

/// Response candidate
#[derive(Debug, Clone, Deserialize)]
pub struct Candidate {
    pub content: Content,
    #[serde(rename = "finishReason")]
    pub finish_reason: Option<String>,
    pub index: i32,
    #[serde(rename = "safetyRatings")]
    pub safety_ratings: Option<Vec<SafetyRating>>,
    #[serde(rename = "citationMetadata")]
    pub citation_metadata: Option<CitationMetadata>,
}

/// Usage metadata
#[derive(Debug, Clone, Deserialize)]
pub struct UsageMetadata {
    #[serde(rename = "promptTokenCount")]
    pub prompt_token_count: i32,
    #[serde(rename = "candidatesTokenCount")]
    pub candidates_token_count: i32,
    #[serde(rename = "totalTokenCount")]
    pub total_token_count: i32,
}

/// Generation configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "maxOutputTokens")]
    pub max_output_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
}

/// Tool definition for function calling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    #[serde(rename = "functionDeclarations")]
    pub function_declarations: Vec<FunctionDeclaration>,
}

/// Tool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    #[serde(rename = "functionCallingConfig")]
    pub function_calling_config: FunctionCallingConfig,
}

/// Function calling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FunctionCallingConfig {
    Auto,
    Any,
    None,
}

/// Function declaration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDeclaration {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,  // JSON Schema
}

/// Function call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub args: serde_json::Value,
}

/// Function response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionResponse {
    pub name: String,
    pub response: serde_json::Value,
}

/// Safety setting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetySetting {
    pub category: HarmCategory,
    pub threshold: HarmBlockThreshold,
}

/// Harm category
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HarmCategory {
    HarmCategoryHarassment,
    HarmCategoryHateSpeech,
    HarmCategorySexuallyExplicit,
    HarmCategoryDangerousContent,
}

/// Harm block threshold
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HarmBlockThreshold {
    BlockNone,
    BlockOnlyHigh,
    BlockMediumAndAbove,
    BlockLowAndAbove,
}

/// Safety rating
#[derive(Debug, Clone, Deserialize)]
pub struct SafetyRating {
    pub category: HarmCategory,
    pub probability: HarmProbability,
    pub blocked: Option<bool>,
}

/// Harm probability
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HarmProbability {
    Negligible,
    Low,
    Medium,
    High,
}

/// Citation metadata
#[derive(Debug, Clone, Deserialize)]
pub struct CitationMetadata {
    pub citations: Vec<Citation>,
}

/// Citation
#[derive(Debug, Clone, Deserialize)]
pub struct Citation {
    #[serde(rename = "startIndex")]
    pub start_index: i32,
    #[serde(rename = "endIndex")]
    pub end_index: i32,
    pub uri: Option<String>,
    pub title: Option<String>,
}

/// Prompt feedback
#[derive(Debug, Clone, Deserialize)]
pub struct PromptFeedback {
    #[serde(rename = "blockReason")]
    pub block_reason: Option<String>,
    #[serde(rename = "safetyRatings")]
    pub safety_ratings: Vec<SafetyRating>,
}

/// Embedding request
#[derive(Debug, Clone, Serialize)]
pub struct EmbedContentRequest {
    pub model: String,
    pub content: Content,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "outputDimensionality")]
    pub output_dimensionality: Option<i32>,
}

/// Embedding response
#[derive(Debug, Clone, Deserialize)]
pub struct EmbedContentResponse {
    pub embedding: Embedding,
}

/// Embedding
#[derive(Debug, Clone, Deserialize)]
pub struct Embedding {
    pub values: Vec<f32>,
}

/// Batch embedding request
#[derive(Debug, Clone, Serialize)]
pub struct BatchEmbedContentsRequest {
    pub requests: Vec<EmbedContentRequest>,
}

/// Batch embedding response
#[derive(Debug, Clone, Deserialize)]
pub struct BatchEmbedContentsResponse {
    pub embeddings: Vec<Embedding>,
}

/// Model info
#[derive(Debug, Clone, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub version: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    pub description: String,
    #[serde(rename = "inputTokenLimit")]
    pub input_token_limit: i32,
    #[serde(rename = "outputTokenLimit")]
    pub output_token_limit: i32,
    #[serde(rename = "supportedGenerationMethods")]
    pub supported_generation_methods: Vec<String>,
    pub temperature: f32,
    #[serde(rename = "topP")]
    pub top_p: f32,
    #[serde(rename = "topK")]
    pub top_k: i32,
}

/// List models response
#[derive(Debug, Clone, Deserialize)]
pub struct ListModelsResponse {
    pub models: Vec<ModelInfo>,
}

/// Token count
#[derive(Debug, Clone, Deserialize)]
pub struct TokenCount {
    #[serde(rename = "totalTokens")]
    pub total_tokens: i32,
}

/// Count tokens request
#[derive(Debug, Clone, Serialize)]
pub struct CountTokensRequest {
    pub contents: Vec<Content>,
}

/// Count tokens response
#[derive(Debug, Clone, Deserialize)]
pub struct CountTokensResponse {
    #[serde(rename = "totalTokens")]
    pub total_tokens: i32,
}
