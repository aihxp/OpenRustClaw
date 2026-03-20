use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use openrustclaw_core::config::{AppConfig, VoiceSttRuntimeConfig};
use openrustclaw_core::types::IncomingMessage;
use reqwest::multipart::{Form, Part};
use serde::Deserialize;
use serde_json::{Map, Value, json};
use tokio::fs;
use tracing::warn;
use uuid::Uuid;

#[derive(Clone)]
pub struct InboundVoiceTranscriber {
    client: reqwest::Client,
    stt: VoiceSttRuntimeConfig,
    api_key_env: String,
    cache_dir: PathBuf,
}

#[derive(Debug, Clone)]
struct AudioCandidate {
    index: usize,
    media_kind: String,
    source: AudioSource,
    filename: String,
    mime: Option<String>,
}

#[derive(Debug, Clone)]
enum AudioSource {
    Local(PathBuf),
    Remote(String),
}

#[derive(Debug, Deserialize)]
struct OpenAiTranscriptionResponse {
    text: String,
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    duration: Option<f64>,
}

#[derive(Debug)]
struct TranscribedAudio {
    text: String,
    language: Option<String>,
    duration_secs: Option<f64>,
    cached_local_path: Option<PathBuf>,
}

impl InboundVoiceTranscriber {
    pub fn try_from_config(config: &AppConfig, workspace_root: &Path) -> Result<Option<Self>> {
        if !config.voice.enabled || !config.voice.stt.transcribe_inbound_notes {
            return Ok(None);
        }

        let provider = config.voice.stt.provider.trim().to_ascii_lowercase();
        if provider != "openai" {
            warn!(
                provider = %config.voice.stt.provider,
                "Inbound voice transcription is enabled, but only the OpenAI-compatible STT lane is currently supported",
            );
            return Ok(None);
        }

        let cache_dir = config
            .voice
            .stt
            .download_dir
            .as_deref()
            .map(|value| resolve_runtime_path(workspace_root, value))
            .unwrap_or_else(|| workspace_root.join(".claw").join("voice").join("inbound"));

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.voice.stt.timeout_secs.max(1)))
            .build()
            .context("failed to build inbound voice transcription HTTP client")?;

        Ok(Some(Self {
            client,
            stt: config.voice.stt.clone(),
            api_key_env: config
                .providers
                .openai
                .api_key_env
                .clone()
                .unwrap_or_else(|| "OPENAI_API_KEY".to_string()),
            cache_dir,
        }))
    }

    pub async fn enrich_incoming_message(&self, mut incoming: IncomingMessage) -> IncomingMessage {
        let Some(candidate) = Self::candidate_from_message(&incoming) else {
            return incoming;
        };

        match self.transcribe_candidate(&candidate).await {
            Ok(transcribed) => {
                if transcribed.text.trim().is_empty() {
                    return incoming;
                }
                apply_transcription(&mut incoming, &candidate, transcribed, &self.stt.model);
            }
            Err(error) => {
                warn!(
                    platform = %incoming.platform,
                    user_id = %incoming.user_id,
                    error = %error,
                    "Failed to transcribe inbound voice note"
                );
                if let Some(object) = incoming.metadata.as_object_mut() {
                    object.insert("voice_has_transcript".into(), json!(false));
                    object.insert("voice_transcription_error".into(), json!(error.to_string()));
                }
            }
        }

        incoming
    }

    fn candidate_from_message(incoming: &IncomingMessage) -> Option<AudioCandidate> {
        let refs = incoming.metadata.get("file_references")?.as_array()?;
        for (index, entry) in refs.iter().enumerate() {
            let media_kind = media_kind_from_entry(entry, &incoming.metadata)?;
            let source = if let Some(path) = entry
                .get("local_path")
                .or_else(|| entry.get("path"))
                .and_then(|value| value.as_str())
                .filter(|value| !value.trim().is_empty())
            {
                AudioSource::Local(PathBuf::from(path))
            } else if let Some(url) = entry
                .get("url")
                .and_then(|value| value.as_str())
                .filter(|value| value.starts_with("http://") || value.starts_with("https://"))
            {
                AudioSource::Remote(url.to_string())
            } else {
                continue;
            };

            let filename = entry
                .get("name")
                .and_then(|value| value.as_str())
                .filter(|value| !value.trim().is_empty())
                .map(ToString::to_string)
                .or_else(|| match &source {
                    AudioSource::Local(path) => path
                        .file_name()
                        .and_then(|value| value.to_str())
                        .map(ToString::to_string),
                    AudioSource::Remote(url) => url
                        .rsplit('/')
                        .next()
                        .and_then(|value| (!value.trim().is_empty()).then_some(value))
                        .map(ToString::to_string),
                })
                .unwrap_or_else(|| default_audio_filename(&media_kind));

            let mime = entry
                .get("mime")
                .or_else(|| entry.get("mime_type"))
                .or_else(|| entry.get("content_type"))
                .and_then(|value| value.as_str())
                .map(ToString::to_string)
                .or_else(|| mime_from_filename(&filename));

            return Some(AudioCandidate {
                index,
                media_kind,
                source,
                filename,
                mime,
            });
        }
        None
    }

    async fn transcribe_candidate(&self, candidate: &AudioCandidate) -> Result<TranscribedAudio> {
        let api_key = std::env::var(&self.api_key_env)
            .with_context(|| format!("{} environment variable not set", self.api_key_env))?;
        let (bytes, cached_local_path) = self.load_candidate_bytes(candidate).await?;

        if bytes.is_empty() {
            return Err(anyhow!("audio payload was empty"));
        }
        if bytes.len() > self.stt.max_audio_bytes {
            return Err(anyhow!(
                "audio payload exceeded max size ({} > {})",
                bytes.len(),
                self.stt.max_audio_bytes
            ));
        }

        let base_url = self
            .stt
            .api_base_url
            .as_deref()
            .unwrap_or("https://api.openai.com/v1")
            .trim_end_matches('/');
        let mut form = Form::new().text("model", self.stt.model.clone()).part(
            "file",
            match candidate.mime.as_deref() {
                Some(mime) => Part::bytes(bytes)
                    .file_name(candidate.filename.clone())
                    .mime_str(mime)
                    .context("failed to attach mime type to audio upload part")?,
                None => Part::bytes(bytes).file_name(candidate.filename.clone()),
            },
        );
        if self.stt.language != "auto" {
            form = form.text("language", self.stt.language.clone());
        }
        if let Some(prompt) = self
            .stt
            .prompt
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            form = form.text("prompt", prompt.to_string());
        }

        let response = self
            .client
            .post(format!("{base_url}/audio/transcriptions"))
            .bearer_auth(api_key)
            .multipart(form)
            .send()
            .await
            .context("failed to call inbound voice transcription endpoint")?;
        let response = response
            .error_for_status()
            .context("inbound voice transcription endpoint returned an error")?;
        let parsed: OpenAiTranscriptionResponse = response
            .json()
            .await
            .context("failed to parse inbound voice transcription response")?;

        Ok(TranscribedAudio {
            text: parsed.text,
            language: parsed.language,
            duration_secs: parsed.duration,
            cached_local_path,
        })
    }

    async fn load_candidate_bytes(
        &self,
        candidate: &AudioCandidate,
    ) -> Result<(Vec<u8>, Option<PathBuf>)> {
        match &candidate.source {
            AudioSource::Local(path) => {
                let bytes = fs::read(path).await.with_context(|| {
                    format!("failed to read local audio file {}", path.display())
                })?;
                Ok((bytes, Some(path.clone())))
            }
            AudioSource::Remote(url) => {
                let response = self
                    .client
                    .get(url)
                    .send()
                    .await
                    .with_context(|| format!("failed to download remote audio file {url}"))?;
                let response = response.error_for_status().with_context(|| {
                    format!("remote audio download returned an error for {url}")
                })?;
                if let Some(content_length) = response.content_length() {
                    if content_length as usize > self.stt.max_audio_bytes {
                        return Err(anyhow!(
                            "remote audio payload exceeded max size ({} > {})",
                            content_length,
                            self.stt.max_audio_bytes
                        ));
                    }
                }
                let bytes = response
                    .bytes()
                    .await
                    .context("failed to read remote audio download body")?;
                let cached_path = self
                    .cache_remote_bytes(candidate, bytes.as_ref())
                    .await
                    .context("failed to cache remote audio payload")?;
                Ok((bytes.to_vec(), Some(cached_path)))
            }
        }
    }

    async fn cache_remote_bytes(
        &self,
        candidate: &AudioCandidate,
        bytes: &[u8],
    ) -> Result<PathBuf> {
        fs::create_dir_all(&self.cache_dir)
            .await
            .with_context(|| format!("failed to create {}", self.cache_dir.display()))?;
        let path = self.cache_dir.join(format!(
            "{}-{}",
            Uuid::new_v4(),
            sanitize_filename(&candidate.filename)
        ));
        fs::write(&path, bytes)
            .await
            .with_context(|| format!("failed to write cached audio file {}", path.display()))?;
        Ok(path)
    }
}

fn apply_transcription(
    incoming: &mut IncomingMessage,
    candidate: &AudioCandidate,
    transcribed: TranscribedAudio,
    model: &str,
) {
    let transcript = transcribed.text.trim();
    if transcript.is_empty() {
        return;
    }

    let mut metadata = incoming
        .metadata
        .as_object()
        .cloned()
        .unwrap_or_else(Map::new);
    metadata.insert("voice_has_transcript".into(), json!(true));
    metadata.insert("voice_transcript".into(), json!(transcript));
    metadata.insert("voice_transcription_provider".into(), json!("openai"));
    metadata.insert("voice_transcription_model".into(), json!(model));
    metadata.insert(
        "voice_transcription_media_kind".into(),
        json!(candidate.media_kind),
    );
    metadata.insert(
        "voice_transcription_filename".into(),
        json!(candidate.filename),
    );
    metadata.insert(
        "voice_transcription_file_reference_index".into(),
        json!(candidate.index),
    );
    if let Some(language) = transcribed
        .language
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        metadata.insert("voice_transcription_language".into(), json!(language));
    }
    if let Some(duration_secs) = transcribed.duration_secs {
        metadata.insert(
            "voice_transcription_duration_secs".into(),
            json!(duration_secs),
        );
    }
    if let Some(local_path) = transcribed.cached_local_path.as_ref() {
        metadata.insert(
            "voice_transcription_local_path".into(),
            json!(local_path.to_string_lossy().to_string()),
        );
        if let Some(file_refs) = metadata
            .get_mut("file_references")
            .and_then(|value| value.as_array_mut())
        {
            if let Some(entry) = file_refs
                .get_mut(candidate.index)
                .and_then(Value::as_object_mut)
            {
                entry.insert(
                    "local_path".into(),
                    json!(local_path.to_string_lossy().to_string()),
                );
            }
        }
    }

    incoming.metadata = Value::Object(metadata);
    incoming.content = append_transcript_to_content(&incoming.content, transcript);
}

fn append_transcript_to_content(existing: &str, transcript: &str) -> String {
    let transcript_block = format!("[Voice transcript]\n{transcript}");
    let trimmed = existing.trim();
    if trimmed.is_empty() {
        return transcript_block;
    }
    if trimmed.contains(transcript) {
        return trimmed.to_string();
    }
    format!("{trimmed}\n\n{transcript_block}")
}

fn media_kind_from_entry(entry: &Value, metadata: &Value) -> Option<String> {
    let explicit_kind = entry
        .get("type")
        .or_else(|| entry.get("media_type"))
        .or_else(|| entry.get("telegram_media_type"))
        .or_else(|| entry.get("whatsapp_media_type"))
        .and_then(|value| value.as_str())
        .map(|value| value.to_ascii_lowercase());
    if let Some(kind) = explicit_kind {
        if is_audio_kind(&kind) {
            return Some(kind);
        }
    }

    if let Some(kind) = metadata
        .get("telegram_media_type")
        .or_else(|| metadata.get("whatsapp_media_type"))
        .or_else(|| metadata.get("signal_media_type"))
        .and_then(|value| value.as_str())
        .map(|value| value.to_ascii_lowercase())
        .filter(|value| is_audio_kind(value))
    {
        return Some(kind);
    }

    let mime = entry
        .get("mime")
        .or_else(|| entry.get("mime_type"))
        .or_else(|| entry.get("content_type"))
        .and_then(|value| value.as_str())
        .map(|value| value.to_ascii_lowercase());
    if let Some(mime) = mime {
        if mime.starts_with("audio/") {
            return Some("audio".to_string());
        }
    }

    let name = entry
        .get("name")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    if let Some(mime) = mime_from_filename(name) {
        if mime.starts_with("audio/") {
            return Some("audio".to_string());
        }
    }

    None
}

fn is_audio_kind(kind: &str) -> bool {
    matches!(kind, "audio" | "voice" | "voice_note" | "voice-note")
}

fn default_audio_filename(media_kind: &str) -> String {
    match media_kind {
        "voice" | "voice_note" | "voice-note" => "voice-note.ogg".to_string(),
        _ => "audio-note.wav".to_string(),
    }
}

fn mime_from_filename(name: &str) -> Option<String> {
    let extension = name.rsplit('.').next()?.to_ascii_lowercase();
    let mime = match extension.as_str() {
        "ogg" | "opus" => "audio/ogg",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "m4a" => "audio/mp4",
        "mp4" => "audio/mp4",
        "webm" => "audio/webm",
        _ => return None,
    };
    Some(mime.to_string())
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn resolve_runtime_path(workspace_root: &Path, value: &str) -> PathBuf {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        path
    } else {
        workspace_root.join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, body::Bytes, extract::State, http::HeaderMap, routing::post};
    use std::net::SocketAddr;
    use std::sync::Arc;
    use tempfile::tempdir;
    use tokio::net::TcpListener;
    use tokio::sync::Mutex;
    use uuid::Uuid;

    #[derive(Clone, Default)]
    struct TestServerState {
        bodies: Arc<Mutex<Vec<String>>>,
    }

    async fn handle_transcription(
        State(state): State<TestServerState>,
        headers: HeaderMap,
        body: Bytes,
    ) -> &'static str {
        assert_eq!(
            headers
                .get(reqwest::header::AUTHORIZATION)
                .and_then(|value| value.to_str().ok()),
            Some("Bearer test-openai-key")
        );
        state
            .bodies
            .lock()
            .await
            .push(String::from_utf8_lossy(body.as_ref()).to_string());
        r#"{"text":"hello from voice","language":"en","duration":3.25}"#
    }

    async fn spawn_test_server() -> (SocketAddr, TestServerState, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind listener");
        let addr = listener.local_addr().expect("local addr");
        let state = TestServerState::default();
        let app = Router::new()
            .route("/audio/transcriptions", post(handle_transcription))
            .with_state(state.clone());
        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve");
        });
        (addr, state, handle)
    }

    fn test_config(base_url: &str, workspace_root: &Path, api_key_env: &str) -> AppConfig {
        let mut config = AppConfig::default();
        config.voice.enabled = true;
        config.voice.stt.transcribe_inbound_notes = true;
        config.voice.stt.api_base_url = Some(base_url.to_string());
        config.voice.stt.download_dir = Some(
            workspace_root
                .join("voice-cache")
                .to_string_lossy()
                .to_string(),
        );
        config.providers.openai.api_key_env = Some(api_key_env.to_string());
        config
    }

    #[tokio::test]
    async fn transcriber_appends_transcript_and_enriches_metadata_from_local_file() {
        let temp = tempdir().expect("tempdir");
        let audio_path = temp.path().join("voice.ogg");
        fs::write(&audio_path, b"fake audio bytes")
            .await
            .expect("write local audio");

        let (addr, state, server_handle) = spawn_test_server().await;
        let config = test_config(
            &format!("http://{}", addr),
            temp.path(),
            "OPENRUSTCLAW_VOICE_TEST_KEY_ONE",
        );
        let transcriber = InboundVoiceTranscriber::try_from_config(&config, temp.path())
            .expect("transcriber config")
            .expect("transcriber");

        let message = IncomingMessage {
            session_id: Uuid::new_v4(),
            user_id: "user-1".to_string(),
            content: "[Voice message]".to_string(),
            platform: openrustclaw_core::types::Platform::Signal,
            metadata: json!({
                "file_references": [{
                    "type": "voice",
                    "mime": "audio/ogg",
                    "name": "voice.ogg",
                    "local_path": audio_path.to_string_lossy().to_string()
                }]
            }),
        };

        unsafe {
            std::env::set_var("OPENRUSTCLAW_VOICE_TEST_KEY_ONE", "test-openai-key");
        }
        let enriched = transcriber.enrich_incoming_message(message).await;
        unsafe {
            std::env::remove_var("OPENRUSTCLAW_VOICE_TEST_KEY_ONE");
        }

        assert!(enriched.content.contains("[Voice transcript]"));
        assert!(enriched.content.contains("hello from voice"));
        assert_eq!(enriched.metadata["voice_has_transcript"], true);
        assert_eq!(enriched.metadata["voice_transcription_language"], "en");
        assert_eq!(
            enriched.metadata["voice_transcription_filename"],
            "voice.ogg"
        );

        let bodies = state.bodies.lock().await;
        assert_eq!(bodies.len(), 1);
        assert!(bodies[0].contains("whisper-1"));

        server_handle.abort();
    }

    #[tokio::test]
    async fn transcriber_downloads_remote_audio_and_sets_local_path() {
        let temp = tempdir().expect("tempdir");
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind listener");
        let addr = listener.local_addr().expect("local addr");
        let state = TestServerState::default();
        let app = Router::new()
            .route(
                "/media/voice.ogg",
                axum::routing::get(|| async { ([("content-type", "audio/ogg")], "ogg-bytes") }),
            )
            .route("/audio/transcriptions", post(handle_transcription))
            .with_state(state.clone());
        let server_handle = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve");
        });

        let config = test_config(
            &format!("http://{}", addr),
            temp.path(),
            "OPENRUSTCLAW_VOICE_TEST_KEY_TWO",
        );
        let transcriber = InboundVoiceTranscriber::try_from_config(&config, temp.path())
            .expect("transcriber config")
            .expect("transcriber");

        let message = IncomingMessage {
            session_id: Uuid::new_v4(),
            user_id: "user-2".to_string(),
            content: "[Voice message]".to_string(),
            platform: openrustclaw_core::types::Platform::WhatsApp,
            metadata: json!({
                "file_references": [{
                    "type": "voice",
                    "mime": "audio/ogg",
                    "name": "voice.ogg",
                    "url": format!("http://{}/media/voice.ogg", addr)
                }]
            }),
        };

        unsafe {
            std::env::set_var("OPENRUSTCLAW_VOICE_TEST_KEY_TWO", "test-openai-key");
        }
        let enriched = transcriber.enrich_incoming_message(message).await;
        unsafe {
            std::env::remove_var("OPENRUSTCLAW_VOICE_TEST_KEY_TWO");
        }

        assert_eq!(enriched.metadata["voice_has_transcript"], true);
        assert!(
            enriched.metadata["voice_transcription_local_path"]
                .as_str()
                .expect("local path")
                .contains("voice.ogg")
        );
        assert!(
            enriched.metadata["file_references"][0]["local_path"]
                .as_str()
                .expect("file ref local path")
                .contains("voice.ogg")
        );

        server_handle.abort();
    }
}
