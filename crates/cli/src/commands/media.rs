use std::path::Path;

use anyhow::{Context, Result, anyhow};
use openrustclaw_automation::browser::{Screenshot, ScreenshotFormat};
use openrustclaw_automation::vision::VisionCapabilities;
use serde::{Deserialize, Serialize};
use tokio::fs;

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
pub struct MediaExtractTextRequest {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaExtractTextResult {
    pub path: String,
    pub media_kind: String,
    pub extractor: String,
    pub text: String,
}

pub async fn inspect(request: MediaInspectRequest) -> Result<()> {
    let result = inspect_data(request).await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

pub async fn extract_text(request: MediaExtractTextRequest) -> Result<()> {
    let result = extract_text_data(request).await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
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
    let text_extractable = is_text_document(path) || media_kind == "image";
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

pub async fn extract_text_data(request: MediaExtractTextRequest) -> Result<MediaExtractTextResult> {
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

    Err(anyhow!(
        "text extraction is not supported for '{}' yet",
        path.display()
    ))
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

fn load_text_preview(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let preview = text.chars().take(280).collect::<String>();
    if text.chars().count() > 280 {
        format!("{preview}...")
    } else {
        preview
    }
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

        let result = extract_text_data(MediaExtractTextRequest {
            path: path.display().to_string(),
        })
        .await
        .expect("extract");

        assert_eq!(result.extractor, "plain_text");
        assert_eq!(result.text, "# heading");
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
}
