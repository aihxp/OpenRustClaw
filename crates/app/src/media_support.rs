use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::Path;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum VisionRequestFormat {
    Anthropic,
    Ollama,
    OpenAiCompatible,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VisionProviderProfile {
    pub request_format: VisionRequestFormat,
}

#[derive(Debug, Clone, Default)]
pub struct MediaSupportService;

impl MediaSupportService {
    pub fn new() -> Self {
        Self
    }

    pub fn build_image_extract_text_prompt(&self, requested_prompt: Option<&str>) -> String {
        requested_prompt
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
            .unwrap_or_else(|| {
                "Extract the visible text from this image. Preserve reading order, line breaks, and short labels where possible. Return only the extracted text.".to_string()
            })
    }

    pub fn build_text_media_describe_prompt(
        &self,
        media_kind: &str,
        requested_prompt: Option<&str>,
        extracted_text: &str,
    ) -> String {
        let default_prompt = match media_kind {
            "audio" => {
                "Summarize the audio artifact from this bounded transcript. Highlight the main points and any action items."
            }
            _ => {
                "Summarize the document artifact from this bounded extracted text. Highlight the main points and any action items."
            }
        };
        let source_label = if media_kind == "audio" {
            "bounded transcript"
        } else {
            "bounded extracted text"
        };
        let excerpt = self.truncate_for_prompt(extracted_text, 16_000);
        format!(
            "{}\n\n{}:\n{}",
            requested_prompt.unwrap_or(default_prompt),
            source_label,
            excerpt
        )
    }

    pub fn build_image_describe_request(
        &self,
        provider: &VisionProviderProfile,
        model: &str,
        max_tokens: u32,
        prompt: &str,
        data_url: &str,
        base64_image: &str,
    ) -> Value {
        match provider.request_format {
            VisionRequestFormat::Anthropic => json!({
                "model": model,
                "max_tokens": max_tokens,
                "messages": [{
                    "role": "user",
                    "content": [
                        { "type": "text", "text": prompt },
                        {
                            "type": "image",
                            "source": {
                                "type": "base64",
                                "media_type": data_url.split(';').next().unwrap_or_default().trim_start_matches("data:"),
                                "data": base64_image
                            }
                        }
                    ]
                }]
            }),
            VisionRequestFormat::Ollama => json!({
                "model": model,
                "stream": false,
                "messages": [{
                    "role": "user",
                    "content": prompt,
                    "images": [base64_image]
                }]
            }),
            VisionRequestFormat::OpenAiCompatible => json!({
                "model": model,
                "messages": [{
                    "role": "user",
                    "content": [
                        { "type": "text", "text": prompt },
                        { "type": "image_url", "image_url": { "url": data_url } }
                    ]
                }],
                "max_tokens": max_tokens,
            }),
        }
    }

    pub fn build_text_describe_request(
        &self,
        provider: &VisionProviderProfile,
        model: &str,
        max_tokens: u32,
        prompt: &str,
    ) -> Value {
        match provider.request_format {
            VisionRequestFormat::Anthropic => json!({
                "model": model,
                "max_tokens": max_tokens,
                "messages": [{
                    "role": "user",
                    "content": [{ "type": "text", "text": prompt }]
                }]
            }),
            VisionRequestFormat::Ollama => json!({
                "model": model,
                "stream": false,
                "messages": [{
                    "role": "user",
                    "content": prompt
                }]
            }),
            VisionRequestFormat::OpenAiCompatible => json!({
                "model": model,
                "messages": [{
                    "role": "user",
                    "content": prompt
                }],
                "max_tokens": max_tokens,
            }),
        }
    }

    pub fn extract_chat_completion_text(&self, payload: &Value) -> Option<String> {
        let content = payload
            .get("choices")?
            .as_array()?
            .first()?
            .get("message")?
            .get("content")?;
        if let Some(text) = content.as_str() {
            let text = text.trim();
            return (!text.is_empty()).then_some(text.to_string());
        }
        let parts = content.as_array()?;
        let text = parts
            .iter()
            .filter_map(|part| part.get("text").and_then(Value::as_str).map(str::trim))
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        (!text.is_empty()).then_some(text)
    }

    pub fn extract_anthropic_text(&self, payload: &Value) -> Option<String> {
        let content = payload.get("content")?.as_array()?;
        let text = content
            .iter()
            .filter_map(|part| {
                (part.get("type").and_then(Value::as_str) == Some("text"))
                    .then(|| part.get("text").and_then(Value::as_str))
                    .flatten()
                    .map(str::trim)
            })
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        (!text.is_empty()).then_some(text)
    }

    pub fn extract_ollama_chat_text(&self, payload: &Value) -> Option<String> {
        payload
            .get("message")?
            .get("content")?
            .as_str()
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(ToString::to_string)
    }

    pub fn normalized_extension(&self, path: &Path) -> Option<String> {
        path.extension()
            .and_then(|value| value.to_str())
            .map(|value| value.trim().to_ascii_lowercase())
    }

    pub fn is_text_document(&self, path: &Path) -> bool {
        matches!(
            self.normalized_extension(path).as_deref(),
            Some("txt" | "md" | "json" | "yaml" | "yml" | "toml" | "csv" | "html" | "xml" | "log")
        )
    }

    pub fn load_text_preview(&self, bytes: &[u8]) -> String {
        let text = String::from_utf8_lossy(bytes);
        let preview = text.chars().take(280).collect::<String>();
        if text.chars().count() > 280 {
            format!("{preview}...")
        } else {
            preview
        }
    }

    fn truncate_for_prompt<'a>(&self, text: &'a str, limit: usize) -> &'a str {
        if text.len() <= limit {
            return text;
        }
        let mut end = limit;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        &text[..end]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_text_media_describe_prompt_uses_kind_specific_defaults() {
        let service = MediaSupportService::new();
        let document_prompt =
            service.build_text_media_describe_prompt("document", None, "alpha\nbeta\n");
        assert!(document_prompt.contains("Summarize the document artifact"));
        assert!(document_prompt.contains("bounded extracted text"));
        let audio_prompt = service.build_text_media_describe_prompt("audio", None, "hello world");
        assert!(audio_prompt.contains("Summarize the audio artifact"));
        assert!(audio_prompt.contains("bounded transcript"));
    }

    #[test]
    fn extract_text_helpers_read_known_response_shapes() {
        let service = MediaSupportService::new();
        assert_eq!(
            service
                .extract_chat_completion_text(&json!({"choices":[{"message":{"content":"alpha"}}]}))
                .as_deref(),
            Some("alpha")
        );
        assert_eq!(
            service
                .extract_anthropic_text(&json!({"content":[{"type":"text","text":"first"},{"type":"text","text":"second"}]}))
                .as_deref(),
            Some("first\nsecond")
        );
    }
}
