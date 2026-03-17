//! Type definitions for the Cloudflare Workers AI API.

use serde::{Deserialize, Serialize};

/// The role of a message author.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// System message.
    System,
    /// User message.
    User,
    /// Assistant message.
    Assistant,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::System => write!(f, "system"),
            Role::User => write!(f, "user"),
            Role::Assistant => write!(f, "assistant"),
        }
    }
}

/// A chat message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// The role of the message author.
    pub role: Role,
    /// The content of the message.
    pub content: String,
}

impl ChatMessage {
    /// Create a new message with the given role and content.
    pub fn new(role: Role, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
        }
    }

    /// Create a system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self::new(Role::System, content)
    }

    /// Create a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self::new(Role::User, content)
    }

    /// Create an assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(Role::Assistant, content)
    }
}

/// Response from a text generation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextGenerationResponse {
    /// The generated text response.
    pub response: String,
}

impl TextGenerationResponse {
    /// Get the generated text.
    pub fn text(&self) -> &str {
        &self.response
    }
}

/// Response from a streaming text generation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextGenerationStreamResponse {
    /// The generated text chunk.
    pub response: String,
}

impl TextGenerationStreamResponse {
    /// Get the text chunk.
    pub fn text(&self) -> &str {
        &self.response
    }

    /// Check if this chunk is empty.
    pub fn is_empty(&self) -> bool {
        self.response.is_empty()
    }
}

/// An embedding vector.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding {
    /// The embedding values.
    pub embedding: Vec<f32>,
}

impl Embedding {
    /// Get the embedding as a slice.
    pub fn as_slice(&self) -> &[f32] {
        &self.embedding
    }
}

/// Response from an embeddings request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingsResponse {
    /// The embeddings for the input(s).
    pub data: Vec<Embedding>,
    /// The shape of the embedding data.
    pub shape: Vec<usize>,
}

impl EmbeddingsResponse {
    /// Get the first embedding.
    pub fn first(&self) -> Option<&Embedding> {
        self.data.first()
    }

    /// Get all embeddings as a Vec of Vec<f32>.
    pub fn embeddings(&self) -> Vec<Vec<f32>> {
        self.data.iter().map(|e| e.embedding.clone()).collect()
    }
}

/// Response from a translation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationResponse {
    /// The translated text.
    pub translated_text: Option<String>,
    /// The detected source language (if auto-detected).
    pub detected_language: Option<String>,
}

impl TranslationResponse {
    /// Get the translated text.
    pub fn text(&self) -> Option<&str> {
        self.translated_text.as_deref()
    }
}

/// Response from a summarization request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummarizationResponse {
    /// The summarized text.
    pub summary: Option<String>,
}

impl SummarizationResponse {
    /// Get the summary text.
    pub fn summary(&self) -> Option<&str> {
        self.summary.as_deref()
    }
}

/// A classification result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationResult {
    /// The predicted label.
    pub label: String,
    /// The confidence score.
    pub score: f32,
}

/// Response from an image classification request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageClassificationResponse {
    /// The classification results.
    pub results: Vec<ClassificationResult>,
}

impl ImageClassificationResponse {
    /// Get the top prediction (highest score).
    pub fn top_prediction(&self) -> Option<&ClassificationResult> {
        self.results.iter().max_by(|a, b| {
            a.score
                .partial_cmp(&b.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    /// Get all results sorted by score (highest first).
    pub fn sorted_results(&self) -> Vec<&ClassificationResult> {
        let mut sorted: Vec<_> = self.results.iter().collect();
        sorted.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted
    }
}

/// Response from a text-to-image request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextToImageResponse {
    /// The base64-encoded image data.
    pub image: Option<String>,
}

impl TextToImageResponse {
    /// Get the base64 image data.
    pub fn base64_image(&self) -> Option<&str> {
        self.image.as_deref()
    }

    /// Decode the base64 image data to bytes.
    pub fn decode_image(&self) -> Result<Vec<u8>, base64::DecodeError> {
        use base64::{Engine, engine::general_purpose::STANDARD};
        match &self.image {
            Some(img) => STANDARD.decode(img),
            None => Ok(Vec::new()),
        }
    }
}

/// Response from a speech recognition request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeechRecognitionResponse {
    /// The transcribed text.
    pub text: Option<String>,
    /// Word-level timestamps (if available).
    pub word_count: Option<usize>,
    /// The detected language.
    pub language: Option<String>,
    /// Duration of the audio in seconds.
    pub duration: Option<f64>,
}

impl SpeechRecognitionResponse {
    /// Get the transcribed text.
    pub fn text(&self) -> Option<&str> {
        self.text.as_deref()
    }
}

/// Cloudflare Workers AI model identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CloudflareModel {
    /// Meta Llama 3 8B Instruct.
    Llama3_8bInstruct,
    /// Meta Llama 3 8B Instruct AWQ (quantized).
    Llama3_8bInstructAwq,
    /// Mistral 7B Instruct v0.1.
    Mistral7bInstruct,
    /// Microsoft Phi-2.
    Phi2,
    /// Qwen 1.5 7B Chat AWQ.
    Qwen15_7bChatAwq,
    /// TinyLlama 1.1B Chat v1.0.
    TinyLlama11bChat,
    /// OpenAI Whisper (speech recognition).
    Whisper,
    /// Stability AI Stable Diffusion XL Base 1.0 (text-to-image).
    StableDiffusionXl,
}

impl CloudflareModel {
    /// Get the model identifier string.
    pub fn as_str(&self) -> &'static str {
        match self {
            CloudflareModel::Llama3_8bInstruct => "@cf/meta/llama-3-8b-instruct",
            CloudflareModel::Llama3_8bInstructAwq => "@cf/meta/llama-3-8b-instruct-awq",
            CloudflareModel::Mistral7bInstruct => "@cf/mistral/mistral-7b-instruct-v0.1",
            CloudflareModel::Phi2 => "@cf/microsoft/phi-2",
            CloudflareModel::Qwen15_7bChatAwq => "@cf/qwen/qwen1.5-7b-chat-awq",
            CloudflareModel::TinyLlama11bChat => "@cf/tinyllama/tinyllama-1.1b-chat-v1.0",
            CloudflareModel::Whisper => "@cf/openai/whisper",
            CloudflareModel::StableDiffusionXl => "@cf/stabilityai/stable-diffusion-xl-base-1.0",
        }
    }

    /// Get the model category/type.
    pub fn category(&self) -> ModelCategory {
        match self {
            CloudflareModel::Llama3_8bInstruct
            | CloudflareModel::Llama3_8bInstructAwq
            | CloudflareModel::Mistral7bInstruct
            | CloudflareModel::Phi2
            | CloudflareModel::Qwen15_7bChatAwq
            | CloudflareModel::TinyLlama11bChat => ModelCategory::TextGeneration,
            CloudflareModel::Whisper => ModelCategory::SpeechRecognition,
            CloudflareModel::StableDiffusionXl => ModelCategory::TextToImage,
        }
    }

    /// Check if this is a text generation model.
    pub fn is_text_generation(&self) -> bool {
        matches!(self.category(), ModelCategory::TextGeneration)
    }

    /// Check if this is a speech recognition model.
    pub fn is_speech(&self) -> bool {
        matches!(self.category(), ModelCategory::SpeechRecognition)
    }

    /// Check if this is an image generation model.
    pub fn is_image(&self) -> bool {
        matches!(self.category(), ModelCategory::TextToImage)
    }
}

impl std::fmt::Display for CloudflareModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for CloudflareModel {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "@cf/meta/llama-3-8b-instruct" => Ok(CloudflareModel::Llama3_8bInstruct),
            "@cf/meta/llama-3-8b-instruct-awq" => Ok(CloudflareModel::Llama3_8bInstructAwq),
            "@cf/mistral/mistral-7b-instruct-v0.1" => Ok(CloudflareModel::Mistral7bInstruct),
            "@cf/microsoft/phi-2" => Ok(CloudflareModel::Phi2),
            "@cf/qwen/qwen1.5-7b-chat-awq" => Ok(CloudflareModel::Qwen15_7bChatAwq),
            "@cf/tinyllama/tinyllama-1.1b-chat-v1.0" => Ok(CloudflareModel::TinyLlama11bChat),
            "@cf/openai/whisper" => Ok(CloudflareModel::Whisper),
            "@cf/stabilityai/stable-diffusion-xl-base-1.0" => {
                Ok(CloudflareModel::StableDiffusionXl)
            }
            _ => Err(format!("Unknown Cloudflare model: {s}")),
        }
    }
}

/// Model categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModelCategory {
    /// Text generation model.
    TextGeneration,
    /// Embeddings model.
    Embeddings,
    /// Translation model.
    Translation,
    /// Summarization model.
    Summarization,
    /// Image classification model.
    ImageClassification,
    /// Text-to-image model.
    TextToImage,
    /// Speech recognition model.
    SpeechRecognition,
}

/// Input for translation requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TranslationInput {
    /// Single text input.
    Single(String),
    /// Multiple text inputs.
    Multiple(Vec<String>),
}

/// Input for summarization requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummarizationInput {
    /// The text to summarize.
    pub text: String,
}

/// Input for image classification requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageClassificationInput {
    /// The base64-encoded image data.
    pub image: Vec<u8>,
}

/// Input for text-to-image requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextToImageInput {
    /// The text prompt.
    pub prompt: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = ChatMessage::user("Hello");
        assert_eq!(msg.role, Role::User);
        assert_eq!(msg.content, "Hello");
    }

    #[test]
    fn test_text_generation_response() {
        let response = TextGenerationResponse {
            response: "Hello world".to_string(),
        };
        assert_eq!(response.text(), "Hello world");
    }

    #[test]
    fn test_model_as_str() {
        assert_eq!(
            CloudflareModel::Llama3_8bInstruct.as_str(),
            "@cf/meta/llama-3-8b-instruct"
        );
        assert_eq!(
            CloudflareModel::Mistral7bInstruct.as_str(),
            "@cf/mistral/mistral-7b-instruct-v0.1"
        );
        assert_eq!(CloudflareModel::Whisper.as_str(), "@cf/openai/whisper");
    }

    #[test]
    fn test_model_from_str() {
        assert_eq!(
            "@cf/meta/llama-3-8b-instruct"
                .parse::<CloudflareModel>()
                .unwrap(),
            CloudflareModel::Llama3_8bInstruct
        );
        assert_eq!(
            "@cf/openai/whisper".parse::<CloudflareModel>().unwrap(),
            CloudflareModel::Whisper
        );
    }

    #[test]
    fn test_model_category() {
        assert!(CloudflareModel::Llama3_8bInstruct.is_text_generation());
        assert!(CloudflareModel::Whisper.is_speech());
        assert!(CloudflareModel::StableDiffusionXl.is_image());
        assert!(!CloudflareModel::Whisper.is_text_generation());
    }

    #[test]
    fn test_embeddings_response() {
        let response = EmbeddingsResponse {
            data: vec![
                Embedding {
                    embedding: vec![0.1, 0.2, 0.3],
                },
                Embedding {
                    embedding: vec![0.4, 0.5, 0.6],
                },
            ],
            shape: vec![2, 3],
        };

        assert_eq!(response.data.len(), 2);
        assert_eq!(response.first().unwrap().embedding.len(), 3);
    }

    #[test]
    fn test_image_classification_response() {
        let response = ImageClassificationResponse {
            results: vec![
                ClassificationResult {
                    label: "cat".to_string(),
                    score: 0.95,
                },
                ClassificationResult {
                    label: "dog".to_string(),
                    score: 0.05,
                },
            ],
        };

        let top = response.top_prediction().unwrap();
        assert_eq!(top.label, "cat");
        assert!((top.score - 0.95).abs() < f32::EPSILON);
    }
}
