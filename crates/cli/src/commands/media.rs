use std::io::{Cursor, Read};
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, anyhow};
use base64::Engine;
use openrustclaw_automation::browser::{Screenshot, ScreenshotFormat};
use openrustclaw_automation::vision::VisionCapabilities;
use openrustclaw_core::config::AppConfig;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::fs;

use super::voice_runtime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaInspectRequest {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaInspectResult {
    pub path: String,
    pub media_kind: String,
    pub mime: String,
    pub bytes: usize,
    #[serde(default)]
    pub image_width: Option<u32>,
    #[serde(default)]
    pub image_height: Option<u32>,
    pub text_extractable: bool,
    pub ocr_available: bool,
    #[serde(default)]
    pub text_preview: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaProviderStatus {
    pub provider: String,
    pub lane: String,
    pub kind: String,
    pub ready: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaProviderCatalog {
    pub extractors: Vec<MediaProviderStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaExtractTextRequest {
    pub path: String,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub max_audio_bytes: Option<usize>,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaExtractTextResult {
    pub path: String,
    pub media_kind: String,
    pub extractor: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaDescribeRequest {
    pub path: String,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaDescribeResult {
    pub path: String,
    pub media_kind: String,
    pub provider: String,
    pub model: String,
    pub description: String,
}

fn load_config(config_path: &str) -> AppConfig {
    AppConfig::load_from(config_path)
        .or_else(|_| AppConfig::load())
        .unwrap_or_default()
}

pub async fn inspect(request: MediaInspectRequest) -> Result<()> {
    let result = inspect_data(request).await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

pub async fn providers(config_path: &str) -> Result<()> {
    let config = load_config(config_path);
    let result = media_provider_catalog(&config);
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

pub async fn extract_text(config_path: &str, request: MediaExtractTextRequest) -> Result<()> {
    let config = load_config(config_path);
    let workspace_root = std::env::current_dir()?;
    let result = extract_text_with_config(&config, &workspace_root, request).await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

pub async fn describe(config_path: &str, request: MediaDescribeRequest) -> Result<()> {
    let config = load_config(config_path);
    let workspace_root = std::env::current_dir()?;
    let result = describe_with_config(&config, &workspace_root, request).await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

pub fn media_provider_catalog(config: &AppConfig) -> MediaProviderCatalog {
    let mut extractors = Vec::new();

    extractors.push(MediaProviderStatus {
        provider: "local".to_string(),
        lane: "plain_text".to_string(),
        kind: "document_text".to_string(),
        ready: true,
        notes: vec!["txt md json yaml toml csv html xml log".to_string()],
    });

    extractors.push(MediaProviderStatus {
        provider: "local".to_string(),
        lane: "docx_xml".to_string(),
        kind: "document_text".to_string(),
        ready: true,
        notes: vec!["bounded_docx_text_extraction".to_string()],
    });

    extractors.push(MediaProviderStatus {
        provider: "local".to_string(),
        lane: "rtf_text".to_string(),
        kind: "document_text".to_string(),
        ready: true,
        notes: vec!["bounded_rtf_text_extraction".to_string()],
    });

    let pdf_ready = pdf_text_extractor_available();
    extractors.push(MediaProviderStatus {
        provider: "local".to_string(),
        lane: "pdf_text".to_string(),
        kind: "document_text".to_string(),
        ready: pdf_ready,
        notes: if pdf_ready {
            vec!["pdftotext_available".to_string()]
        } else {
            vec!["pdftotext_unavailable".to_string()]
        },
    });

    let ocr_ready = VisionCapabilities::new().ocr_enabled();
    extractors.push(MediaProviderStatus {
        provider: "local".to_string(),
        lane: "ocr".to_string(),
        kind: "image_text".to_string(),
        ready: ocr_ready,
        notes: if ocr_ready {
            vec!["local_ocr_available".to_string()]
        } else {
            vec!["local_ocr_unavailable".to_string()]
        },
    });

    for provider in voice_runtime::voice_provider_catalog(config).stt {
        let mut notes = provider.notes;
        if provider.supports_inbound_notes {
            notes.push("usable_for_bounded_audio_text_extraction".to_string());
        }
        extractors.push(MediaProviderStatus {
            provider: provider.provider,
            lane: provider.lane,
            kind: "audio_text".to_string(),
            ready: provider.api_key_present,
            notes,
        });
    }

    extractors.extend(vision_provider_catalog(config));

    MediaProviderCatalog { extractors }
}

pub async fn inspect_data(request: MediaInspectRequest) -> Result<MediaInspectResult> {
    let path = Path::new(&request.path);
    if !path.exists() {
        return Err(anyhow!("path '{}' does not exist", path.display()));
    }
    let bytes = fs::read(path)
        .await
        .with_context(|| format!("failed to read {}", path.display()))?;
    let media_kind = detect_media_kind(path);
    let mime = guess_mime(path);
    let mut image_width = None;
    let mut image_height = None;
    if media_kind == "image" {
        let image = image::load_from_memory(&bytes)
            .with_context(|| format!("failed to decode image '{}'", path.display()))?;
        image_width = Some(image.width());
        image_height = Some(image.height());
    }
    let text_extractable = is_text_document(path)
        || is_rich_text_document(path)
        || matches!(media_kind.as_str(), "image" | "audio");
    let text_preview = if is_text_document(path) {
        Some(load_text_preview(&bytes))
    } else {
        None
    };
    let ocr_available = VisionCapabilities::new().ocr_enabled();

    Ok(MediaInspectResult {
        path: path.display().to_string(),
        media_kind,
        mime,
        bytes: bytes.len(),
        image_width,
        image_height,
        text_extractable,
        ocr_available,
        text_preview,
    })
}

pub async fn extract_text_with_config(
    config: &AppConfig,
    workspace_root: &Path,
    request: MediaExtractTextRequest,
) -> Result<MediaExtractTextResult> {
    let path = Path::new(&request.path);
    if !path.exists() {
        return Err(anyhow!("path '{}' does not exist", path.display()));
    }
    let bytes = fs::read(path)
        .await
        .with_context(|| format!("failed to read {}", path.display()))?;
    let media_kind = detect_media_kind(path);

    if is_text_document(path) {
        let text = String::from_utf8(bytes)
            .with_context(|| format!("failed to decode text document '{}'", path.display()))?;
        return Ok(MediaExtractTextResult {
            path: path.display().to_string(),
            media_kind: "document".to_string(),
            extractor: "plain_text".to_string(),
            text,
        });
    }

    if is_docx_document(path) {
        let text = extract_docx_text(&bytes)
            .with_context(|| format!("failed to extract docx text from '{}'", path.display()))?;
        return Ok(MediaExtractTextResult {
            path: path.display().to_string(),
            media_kind: "document".to_string(),
            extractor: "docx_xml".to_string(),
            text,
        });
    }

    if is_rtf_document(path) {
        let text = extract_rtf_text(&bytes)
            .with_context(|| format!("failed to extract rtf text from '{}'", path.display()))?;
        return Ok(MediaExtractTextResult {
            path: path.display().to_string(),
            media_kind: "document".to_string(),
            extractor: "rtf_text".to_string(),
            text,
        });
    }

    if is_pdf_document(path) {
        let text = extract_pdf_text(path)
            .with_context(|| format!("failed to extract pdf text from '{}'", path.display()))?;
        return Ok(MediaExtractTextResult {
            path: path.display().to_string(),
            media_kind: "document".to_string(),
            extractor: "pdf_text".to_string(),
            text,
        });
    }

    if media_kind == "image" {
        let image = image::load_from_memory(&bytes)
            .with_context(|| format!("failed to decode image '{}'", path.display()))?;
        let screenshot = Screenshot {
            data: bytes,
            width: image.width(),
            height: image.height(),
            format: ScreenshotFormat::Png,
        };
        let text = VisionCapabilities::new()
            .extract_text(&screenshot)
            .await
            .map_err(|error| anyhow!(error.to_string()))?;
        return Ok(MediaExtractTextResult {
            path: path.display().to_string(),
            media_kind,
            extractor: "ocr".to_string(),
            text,
        });
    }

    if media_kind == "audio" {
        let result = voice_runtime::transcribe_with_config(
            config,
            workspace_root,
            voice_runtime::VoiceTranscribeRequest {
                path: Some(path.display().to_string()),
                url: None,
                provider: request.provider,
                model: request.model,
                language: request.language,
                prompt: request.prompt,
                max_audio_bytes: request.max_audio_bytes,
                timeout_secs: request.timeout_secs,
            },
        )
        .await?;
        return Ok(MediaExtractTextResult {
            path: path.display().to_string(),
            media_kind,
            extractor: format!("audio_stt:{}", result.provider),
            text: result.text,
        });
    }

    Err(anyhow!(
        "text extraction is not supported for '{}' yet",
        path.display()
    ))
}

pub async fn describe_with_config(
    config: &AppConfig,
    workspace_root: &Path,
    request: MediaDescribeRequest,
) -> Result<MediaDescribeResult> {
    let path = Path::new(&request.path);
    if !path.exists() {
        return Err(anyhow!("path '{}' does not exist", path.display()));
    }
    let bytes = fs::read(path)
        .await
        .with_context(|| format!("failed to read {}", path.display()))?;
    let media_kind = detect_media_kind(path);

    let provider = resolve_vision_provider(config, request.provider.as_deref())?;
    let model = request
        .model
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| provider.default_model.clone());
    let prompt = cleaned_optional_text(request.prompt.as_deref());
    let description = match media_kind.as_str() {
        "image" => {
            let image_prompt =
                prompt.unwrap_or_else(|| "Describe this image in detail.".to_string());
            let body = build_image_describe_request(
                &provider,
                &model,
                request.max_tokens.unwrap_or(600),
                &image_prompt,
                &bytes,
                &guess_mime(path),
            );
            call_media_describe_provider(&provider, &body).await?
        }
        "document" | "audio" => {
            let extracted = extract_text_with_config(
                config,
                workspace_root,
                MediaExtractTextRequest {
                    path: path.display().to_string(),
                    provider: None,
                    model: None,
                    language: None,
                    prompt: None,
                    max_audio_bytes: None,
                    timeout_secs: None,
                },
            )
            .await?;
            let text_prompt = build_text_media_describe_prompt(
                media_kind.as_str(),
                prompt.as_deref(),
                &extracted.text,
            );
            let body = build_text_describe_request(
                &provider,
                &model,
                request.max_tokens.unwrap_or(600),
                &text_prompt,
            );
            call_media_describe_provider(&provider, &body).await?
        }
        _ => {
            return Err(anyhow!(
                "bounded provider-backed describe currently supports image, document, and audio artifacts only"
            ));
        }
    };

    Ok(MediaDescribeResult {
        path: path.display().to_string(),
        media_kind,
        provider: provider.provider,
        model,
        description,
    })
}

fn detect_media_kind(path: &Path) -> String {
    match normalized_extension(path).as_deref() {
        Some("png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp") => "image".to_string(),
        Some("mp3" | "wav" | "ogg" | "opus" | "flac" | "m4a") => "audio".to_string(),
        Some("txt" | "md" | "json" | "yaml" | "yml" | "toml" | "csv" | "html" | "xml" | "log")
        | Some("pdf" | "doc" | "docx" | "rtf") => "document".to_string(),
        _ => "unknown".to_string(),
    }
}

fn guess_mime(path: &Path) -> String {
    match normalized_extension(path).as_deref() {
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("bmp") => "image/bmp",
        Some("mp3") => "audio/mpeg",
        Some("wav") => "audio/wav",
        Some("ogg") => "audio/ogg",
        Some("opus") => "audio/ogg",
        Some("flac") => "audio/flac",
        Some("m4a") => "audio/mp4",
        Some("txt" | "log") => "text/plain",
        Some("md") => "text/markdown",
        Some("json") => "application/json",
        Some("yaml" | "yml") => "application/yaml",
        Some("toml") => "application/toml",
        Some("csv") => "text/csv",
        Some("html") => "text/html",
        Some("xml") => "application/xml",
        Some("pdf") => "application/pdf",
        Some("doc") => "application/msword",
        Some("docx") => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        Some("rtf") => "application/rtf",
        _ => "application/octet-stream",
    }
    .to_string()
}

#[derive(Debug, Clone)]
struct VisionProviderProfile {
    provider: String,
    default_model: String,
    api_base_url: String,
    api_key_env: Option<String>,
    api_version: Option<String>,
    request_format: VisionRequestFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VisionRequestFormat {
    OpenAiCompatible,
    Anthropic,
    Ollama,
}

fn vision_provider_catalog(config: &AppConfig) -> Vec<MediaProviderStatus> {
    let mut extractors = Vec::new();
    for (provider, lane, model, api_key_env, api_base_url, note) in [
        (
            "ollama",
            "ollama_local_vision",
            config.providers.ollama.model.clone(),
            None,
            config.providers.ollama.base_url.clone(),
            "local_api_chat_image_understanding".to_string(),
        ),
        (
            "anthropic",
            "anthropic_vision",
            config.providers.anthropic.model.clone(),
            config.providers.anthropic.api_key_env.clone(),
            "https://api.anthropic.com".to_string(),
            "messages_api_image_understanding".to_string(),
        ),
        (
            "openai",
            "openai_compatible_vision",
            config.providers.openai.model.clone(),
            config.providers.openai.api_key_env.clone(),
            "https://api.openai.com".to_string(),
            "chat_completions_image_understanding".to_string(),
        ),
        (
            "openrouter",
            "openrouter_vision",
            config.providers.openrouter.model.clone(),
            config.providers.openrouter.api_key_env.clone(),
            "https://openrouter.ai/api".to_string(),
            "openai_compatible_image_understanding".to_string(),
        ),
    ] {
        let ready = if provider == "ollama" {
            !api_base_url.trim().is_empty() && !model.trim().is_empty()
        } else {
            api_key_env
                .as_deref()
                .and_then(|env| std::env::var(env).ok())
                .is_some()
        };
        let mut notes = vec![
            note,
            format!("default_model:{model}"),
            format!("base_url:{api_base_url}"),
        ];
        if provider == "ollama" {
            notes.push("runtime_probe_required".to_string());
        } else if !ready {
            notes.push("api_key_unavailable".to_string());
        }
        extractors.push(MediaProviderStatus {
            provider: provider.to_string(),
            lane: lane.to_string(),
            kind: "image_description".to_string(),
            ready,
            notes: notes.clone(),
        });
        extractors.push(MediaProviderStatus {
            provider: provider.to_string(),
            lane: format!("{lane}_document"),
            kind: "document_description".to_string(),
            ready,
            notes: notes.clone(),
        });
        extractors.push(MediaProviderStatus {
            provider: provider.to_string(),
            lane: format!("{lane}_audio"),
            kind: "audio_description".to_string(),
            ready,
            notes,
        });
    }
    extractors
}

fn resolve_vision_provider(
    config: &AppConfig,
    requested: Option<&str>,
) -> Result<VisionProviderProfile> {
    let provider = requested
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| match config.providers.default_provider.as_str() {
            "anthropic" | "openai" | "openrouter" | "ollama" => {
                config.providers.default_provider.clone()
            }
            _ if config
                .providers
                .openai
                .api_key_env
                .as_deref()
                .and_then(|env| std::env::var(env).ok())
                .is_some() =>
            {
                "openai".to_string()
            }
            _ if config
                .providers
                .anthropic
                .api_key_env
                .as_deref()
                .and_then(|env| std::env::var(env).ok())
                .is_some() =>
            {
                "anthropic".to_string()
            }
            _ if !config.providers.ollama.base_url.trim().is_empty()
                && !config.providers.ollama.model.trim().is_empty() =>
            {
                "ollama".to_string()
            }
            _ => "openrouter".to_string(),
        });
    match provider.as_str() {
        "ollama" => Ok(VisionProviderProfile {
            provider,
            default_model: config.providers.ollama.model.clone(),
            api_base_url: config
                .providers
                .ollama
                .base_url
                .trim_end_matches('/')
                .to_string(),
            api_key_env: None,
            api_version: None,
            request_format: VisionRequestFormat::Ollama,
        }),
        "anthropic" => Ok(VisionProviderProfile {
            provider,
            default_model: config.providers.anthropic.model.clone(),
            api_base_url: "https://api.anthropic.com".to_string(),
            api_key_env: Some(
                config
                    .providers
                    .anthropic
                    .api_key_env
                    .clone()
                    .ok_or_else(|| anyhow!("providers.anthropic.api_key_env is not configured"))?,
            ),
            api_version: Some(config.providers.anthropic.api_version.clone()),
            request_format: VisionRequestFormat::Anthropic,
        }),
        "openai" => Ok(VisionProviderProfile {
            provider,
            default_model: config.providers.openai.model.clone(),
            api_base_url: "https://api.openai.com".to_string(),
            api_key_env: Some(
                config
                    .providers
                    .openai
                    .api_key_env
                    .clone()
                    .ok_or_else(|| anyhow!("providers.openai.api_key_env is not configured"))?,
            ),
            api_version: None,
            request_format: VisionRequestFormat::OpenAiCompatible,
        }),
        "openrouter" => {
            Ok(VisionProviderProfile {
                provider,
                default_model: config.providers.openrouter.model.clone(),
                api_base_url: "https://openrouter.ai/api".to_string(),
                api_key_env: Some(config.providers.openrouter.api_key_env.clone().ok_or_else(
                    || anyhow!("providers.openrouter.api_key_env is not configured"),
                )?),
                api_version: None,
                request_format: VisionRequestFormat::OpenAiCompatible,
            })
        }
        other => Err(anyhow!(
            "unsupported bounded media vision provider '{}'; supported providers: anthropic, ollama, openai, openrouter",
            other
        )),
    }
}

fn extract_chat_completion_text(payload: &serde_json::Value) -> Option<String> {
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
        .filter_map(|part| {
            part.get("text")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
        })
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    (!text.is_empty()).then_some(text)
}

fn cleaned_optional_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn build_text_media_describe_prompt(
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
    let excerpt = truncate_for_prompt(extracted_text, 16_000);
    format!(
        "{}\n\n{}:\n{}",
        requested_prompt.unwrap_or(default_prompt),
        source_label,
        excerpt
    )
}

fn build_image_describe_request(
    provider: &VisionProviderProfile,
    model: &str,
    max_tokens: u32,
    prompt: &str,
    bytes: &[u8],
    mime: &str,
) -> serde_json::Value {
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
                            "media_type": mime,
                            "data": base64::engine::general_purpose::STANDARD.encode(bytes)
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
                "images": [
                    base64::engine::general_purpose::STANDARD.encode(bytes)
                ]
            }]
        }),
        VisionRequestFormat::OpenAiCompatible => {
            let data_url = format!(
                "data:{};base64,{}",
                mime,
                base64::engine::general_purpose::STANDARD.encode(bytes)
            );
            json!({
                "model": model,
                "messages": [{
                    "role": "user",
                    "content": [
                        { "type": "text", "text": prompt },
                        { "type": "image_url", "image_url": { "url": data_url } }
                    ]
                }],
                "max_tokens": max_tokens,
            })
        }
    }
}

fn build_text_describe_request(
    provider: &VisionProviderProfile,
    model: &str,
    max_tokens: u32,
    prompt: &str,
) -> serde_json::Value {
    match provider.request_format {
        VisionRequestFormat::Anthropic => json!({
            "model": model,
            "max_tokens": max_tokens,
            "messages": [{
                "role": "user",
                "content": [
                    { "type": "text", "text": prompt }
                ]
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

fn truncate_for_prompt(text: &str, limit: usize) -> &str {
    if text.len() <= limit {
        return text;
    }
    let mut end = limit;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

async fn call_media_describe_provider(
    provider: &VisionProviderProfile,
    body: &serde_json::Value,
) -> Result<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .context("failed to build media describe HTTP client")?;
    let mut request_builder =
        match provider.request_format {
            VisionRequestFormat::Ollama => client
                .post(format!("{}/api/chat", provider.api_base_url))
                .header(reqwest::header::CONTENT_TYPE, "application/json"),
            VisionRequestFormat::Anthropic => client
                .post(format!("{}/v1/messages", provider.api_base_url))
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .header(
                    "x-api-key",
                    std::env::var(provider.api_key_env.as_deref().ok_or_else(|| {
                        anyhow!("providers.anthropic.api_key_env is not configured")
                    })?)
                    .with_context(|| {
                        format!(
                            "{} environment variable not set",
                            provider.api_key_env.as_deref().unwrap_or_default()
                        )
                    })?,
                )
                .header(
                    "anthropic-version",
                    provider.api_version.as_deref().unwrap_or("2023-06-01"),
                ),
            VisionRequestFormat::OpenAiCompatible => client
                .post(format!("{}/v1/chat/completions", provider.api_base_url))
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .bearer_auth(
                    std::env::var(
                        provider
                            .api_key_env
                            .as_deref()
                            .ok_or_else(|| anyhow!("provider API key env is not configured"))?,
                    )
                    .with_context(|| {
                        format!(
                            "{} environment variable not set",
                            provider.api_key_env.as_deref().unwrap_or_default()
                        )
                    })?,
                ),
        };
    if provider.provider == "openrouter" {
        request_builder = request_builder
            .header("HTTP-Referer", "https://openrustclaw.dev")
            .header("X-Title", "OpenRustClaw");
    }

    let response = request_builder
        .json(body)
        .send()
        .await
        .context("failed to call provider-backed media describe lane")?;
    let status = response.status();
    let payload: serde_json::Value = response
        .json()
        .await
        .context("failed to parse media describe response")?;
    if !status.is_success() {
        return Err(anyhow!(
            "media describe provider '{}' returned {}: {}",
            provider.provider,
            status,
            payload
        ));
    }
    match provider.request_format {
        VisionRequestFormat::Ollama => extract_ollama_chat_text(&payload)
            .ok_or_else(|| anyhow!("provider-backed media describe response did not contain text")),
        VisionRequestFormat::Anthropic => extract_anthropic_text(&payload)
            .ok_or_else(|| anyhow!("provider-backed media describe response did not contain text")),
        VisionRequestFormat::OpenAiCompatible => extract_chat_completion_text(&payload)
            .ok_or_else(|| anyhow!("provider-backed media describe response did not contain text")),
    }
}

fn extract_ollama_chat_text(payload: &serde_json::Value) -> Option<String> {
    payload
        .get("message")?
        .get("content")?
        .as_str()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToString::to_string)
}

fn extract_anthropic_text(payload: &serde_json::Value) -> Option<String> {
    let content = payload.get("content")?.as_array()?;
    let text = content
        .iter()
        .filter_map(|part| {
            (part.get("type").and_then(serde_json::Value::as_str) == Some("text"))
                .then(|| part.get("text").and_then(serde_json::Value::as_str))
                .flatten()
                .map(str::trim)
        })
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    (!text.is_empty()).then_some(text)
}

fn normalized_extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|value| value.trim().to_ascii_lowercase())
}

fn is_text_document(path: &Path) -> bool {
    matches!(
        normalized_extension(path).as_deref(),
        Some("txt" | "md" | "json" | "yaml" | "yml" | "toml" | "csv" | "html" | "xml" | "log")
    )
}

fn is_rich_text_document(path: &Path) -> bool {
    is_docx_document(path) || is_rtf_document(path) || is_pdf_document(path)
}

fn is_docx_document(path: &Path) -> bool {
    matches!(normalized_extension(path).as_deref(), Some("docx"))
}

fn is_rtf_document(path: &Path) -> bool {
    matches!(normalized_extension(path).as_deref(), Some("rtf"))
}

fn is_pdf_document(path: &Path) -> bool {
    matches!(normalized_extension(path).as_deref(), Some("pdf"))
}

fn load_text_preview(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let preview = text.chars().take(280).collect::<String>();
    if text.chars().count() > 280 {
        format!("{preview}...")
    } else {
        preview
    }
}

fn pdf_text_extractor_available() -> bool {
    Command::new("pdftotext")
        .arg("-v")
        .output()
        .map(|output| output.status.success() || !output.stderr.is_empty())
        .unwrap_or(false)
}

fn extract_docx_text(bytes: &[u8]) -> Result<String> {
    let cursor = Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor).context("failed to open docx zip archive")?;
    let mut document = archive
        .by_name("word/document.xml")
        .context("docx missing word/document.xml")?;
    let mut xml = String::new();
    document
        .read_to_string(&mut xml)
        .context("failed to read docx document.xml")?;
    Ok(clean_whitespace(&decode_xml_entities(
        &strip_xml_tags_with_breaks(&xml),
    )))
}

fn extract_rtf_text(bytes: &[u8]) -> Result<String> {
    let raw = String::from_utf8(bytes.to_vec()).context("rtf bytes are not valid utf-8")?;
    let mut output = String::new();
    let mut chars = raw.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '{' | '}' => {}
            '\\' => {
                if let Some('\'') = chars.peek().copied() {
                    chars.next();
                    let hi = chars.next().unwrap_or('0');
                    let lo = chars.next().unwrap_or('0');
                    if let Ok(byte) = u8::from_str_radix(&format!("{hi}{lo}"), 16) {
                        output.push(byte as char);
                    }
                    continue;
                }
                let mut control = String::new();
                while let Some(next) = chars.peek().copied() {
                    if next.is_ascii_alphabetic() {
                        control.push(next);
                        chars.next();
                    } else {
                        break;
                    }
                }
                while let Some(next) = chars.peek().copied() {
                    if next.is_ascii_digit() || next == '-' {
                        chars.next();
                    } else {
                        break;
                    }
                }
                if chars.peek() == Some(&' ') {
                    chars.next();
                }
                match control.as_str() {
                    "par" | "line" => output.push('\n'),
                    "tab" => output.push('\t'),
                    _ => {}
                }
            }
            '\r' | '\n' => {}
            other => output.push(other),
        }
    }
    Ok(clean_whitespace(&output))
}

fn extract_pdf_text(path: &Path) -> Result<String> {
    if !pdf_text_extractor_available() {
        return Err(anyhow!(
            "local pdf text extractor 'pdftotext' is unavailable"
        ));
    }
    let output = Command::new("pdftotext")
        .arg("-layout")
        .arg(path)
        .arg("-")
        .output()
        .with_context(|| format!("failed to execute pdftotext for '{}'", path.display()))?;
    if !output.status.success() {
        return Err(anyhow!(
            "pdftotext failed for '{}': {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(clean_whitespace(&String::from_utf8_lossy(&output.stdout)))
}

fn strip_xml_tags_with_breaks(xml: &str) -> String {
    let with_breaks = xml
        .replace("</w:p>", "\n")
        .replace("</w:tr>", "\n")
        .replace("<w:tab/>", "\t");
    let mut output = String::with_capacity(with_breaks.len());
    let mut in_tag = false;
    for ch in with_breaks.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => output.push(ch),
            _ => {}
        }
    }
    output
}

fn decode_xml_entities(input: &str) -> String {
    input
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#9;", "\t")
        .replace("&#10;", "\n")
        .replace("&#13;", "\n")
}

fn clean_whitespace(input: &str) -> String {
    input
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn inspect_text_document_reports_preview() {
        let temp = tempdir().expect("tempdir");
        let path = temp.path().join("notes.txt");
        fs::write(&path, "hello media lane")
            .await
            .expect("write text");

        let result = inspect_data(MediaInspectRequest {
            path: path.display().to_string(),
        })
        .await
        .expect("inspect");

        assert_eq!(result.media_kind, "document");
        assert_eq!(result.mime, "text/plain");
        assert!(result.text_extractable);
        assert_eq!(result.text_preview.as_deref(), Some("hello media lane"));
    }

    #[tokio::test]
    async fn extract_text_reads_plain_text_documents() {
        let temp = tempdir().expect("tempdir");
        let path = temp.path().join("notes.md");
        fs::write(&path, "# heading").await.expect("write text");

        let result = extract_text_with_config(
            &AppConfig::default(),
            temp.path(),
            MediaExtractTextRequest {
                path: path.display().to_string(),
                provider: None,
                model: None,
                language: None,
                prompt: None,
                max_audio_bytes: None,
                timeout_secs: None,
            },
        )
        .await
        .expect("extract");

        assert_eq!(result.extractor, "plain_text");
        assert_eq!(result.text, "# heading");
    }

    #[tokio::test]
    async fn extract_text_reads_rtf_documents() {
        let temp = tempdir().expect("tempdir");
        let path = temp.path().join("notes.rtf");
        fs::write(&path, "{\\rtf1\\ansi hello\\par world}")
            .await
            .expect("write rtf");

        let result = extract_text_with_config(
            &AppConfig::default(),
            temp.path(),
            MediaExtractTextRequest {
                path: path.display().to_string(),
                provider: None,
                model: None,
                language: None,
                prompt: None,
                max_audio_bytes: None,
                timeout_secs: None,
            },
        )
        .await
        .expect("extract rtf");

        assert_eq!(result.extractor, "rtf_text");
        assert_eq!(result.text, "hello\nworld");
    }

    #[tokio::test]
    async fn extract_text_reads_docx_documents() {
        let temp = tempdir().expect("tempdir");
        let path = temp.path().join("notes.docx");
        let cursor = Cursor::new(Vec::new());
        let mut writer = zip::ZipWriter::new(cursor);
        let options = zip::write::SimpleFileOptions::default();
        writer
            .start_file("word/document.xml", options)
            .expect("start docx xml");
        std::io::Write::write_all(
            &mut writer,
            br#"<?xml version="1.0" encoding="UTF-8"?><w:document><w:body><w:p><w:r><w:t>Hello</w:t></w:r></w:p><w:p><w:r><w:t>World</w:t></w:r></w:p></w:body></w:document>"#,
        )
        .expect("write docx xml");
        let bytes = writer.finish().expect("finish zip").into_inner();
        fs::write(&path, bytes).await.expect("write docx");

        let result = extract_text_with_config(
            &AppConfig::default(),
            temp.path(),
            MediaExtractTextRequest {
                path: path.display().to_string(),
                provider: None,
                model: None,
                language: None,
                prompt: None,
                max_audio_bytes: None,
                timeout_secs: None,
            },
        )
        .await
        .expect("extract docx");

        assert_eq!(result.extractor, "docx_xml");
        assert_eq!(result.text, "Hello\nWorld");
    }

    #[tokio::test]
    async fn inspect_image_reports_dimensions() {
        let temp = tempdir().expect("tempdir");
        let path = temp.path().join("pixel.png");
        let image = image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 0, 0, 255]));
        image.save(&path).expect("write png");

        let result = inspect_data(MediaInspectRequest {
            path: path.display().to_string(),
        })
        .await
        .expect("inspect image");

        assert_eq!(result.media_kind, "image");
        assert_eq!(result.mime, "image/png");
        assert_eq!(result.image_width, Some(1));
        assert_eq!(result.image_height, Some(1));
    }

    #[test]
    fn provider_catalog_includes_audio_stt_and_local_extractors() {
        let mut config = AppConfig::default();
        config.providers.anthropic.api_key_env = Some("ANTHROPIC_TEST_KEY".to_string());
        config.providers.openai.api_key_env = Some("OPENAI_TEST_KEY".to_string());
        config.providers.openrouter.api_key_env = Some("OPENROUTER_TEST_KEY".to_string());
        let catalog = media_provider_catalog(&config);

        assert!(
            catalog
                .extractors
                .iter()
                .any(|entry| entry.kind == "document_text" && entry.provider == "local")
        );
        assert!(
            catalog
                .extractors
                .iter()
                .any(|entry| entry.kind == "image_text" && entry.provider == "local")
        );
        assert!(
            catalog
                .extractors
                .iter()
                .any(|entry| entry.kind == "audio_text" && entry.provider == "openai")
        );
        assert!(
            catalog
                .extractors
                .iter()
                .any(|entry| entry.lane == "docx_xml" && entry.kind == "document_text")
        );
        assert!(
            catalog
                .extractors
                .iter()
                .any(|entry| entry.lane == "rtf_text" && entry.kind == "document_text")
        );
        assert!(
            catalog
                .extractors
                .iter()
                .any(|entry| entry.kind == "image_description" && entry.provider == "ollama")
        );
        assert!(
            catalog
                .extractors
                .iter()
                .any(|entry| entry.kind == "image_description" && entry.provider == "anthropic")
        );
        assert!(
            catalog
                .extractors
                .iter()
                .any(|entry| entry.kind == "image_description" && entry.provider == "openai")
        );
        assert!(
            catalog
                .extractors
                .iter()
                .any(|entry| entry.kind == "image_description" && entry.provider == "openrouter")
        );
        assert!(
            catalog
                .extractors
                .iter()
                .any(|entry| entry.kind == "document_description" && entry.provider == "openai")
        );
        assert!(
            catalog
                .extractors
                .iter()
                .any(|entry| entry.kind == "audio_description" && entry.provider == "openrouter")
        );
    }

    #[test]
    fn extract_chat_completion_text_reads_string_content() {
        let payload = json!({
            "choices": [{
                "message": {
                    "content": "A red square on a transparent background."
                }
            }]
        });
        assert_eq!(
            extract_chat_completion_text(&payload).as_deref(),
            Some("A red square on a transparent background.")
        );
    }

    #[test]
    fn extract_anthropic_text_reads_content_array() {
        let payload = json!({
            "content": [
                { "type": "text", "text": "First line." },
                { "type": "tool_use", "id": "toolu_123", "name": "ignored" },
                { "type": "text", "text": "Second line." }
            ]
        });
        assert_eq!(
            extract_anthropic_text(&payload).as_deref(),
            Some("First line.\nSecond line.")
        );
    }

    #[test]
    fn extract_ollama_chat_text_reads_message_content() {
        let payload = json!({
            "message": {
                "role": "assistant",
                "content": "Local vision summary."
            }
        });
        assert_eq!(
            extract_ollama_chat_text(&payload).as_deref(),
            Some("Local vision summary.")
        );
    }

    #[test]
    fn build_text_media_describe_prompt_uses_kind_specific_defaults() {
        let document_prompt = build_text_media_describe_prompt("document", None, "alpha\nbeta\n");
        assert!(document_prompt.contains("Summarize the document artifact"));
        assert!(document_prompt.contains("bounded extracted text"));
        assert!(document_prompt.contains("alpha\nbeta"));

        let audio_prompt = build_text_media_describe_prompt("audio", None, "hello world");
        assert!(audio_prompt.contains("Summarize the audio artifact"));
        assert!(audio_prompt.contains("bounded transcript"));
    }
}
