use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use openrustclaw_core::config::{AppConfig, VoiceSttRuntimeConfig, VoiceTtsRuntimeConfig};
use openrustclaw_core::types::IncomingMessage;
use reqwest::Url;
use reqwest::multipart::{Form, Part};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use tokio::fs;
use tracing::warn;
use uuid::Uuid;

#[derive(Clone)]
pub struct InboundVoiceTranscriber {
    client: reqwest::Client,
    provider: String,
    stt: VoiceSttRuntimeConfig,
    api_key_env: String,
    api_base_url: String,
    cache_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceStatus {
    pub enabled: bool,
    pub stt_provider: String,
    pub stt_model: String,
    pub stt_language: String,
    pub transcribe_inbound_notes: bool,
    pub download_dir: Option<String>,
    pub timeout_secs: u64,
    pub max_audio_bytes: usize,
    pub api_base_url: Option<String>,
    pub api_key_env: String,
    pub api_key_present: bool,
    pub tts_provider: String,
    pub tts_model: String,
    pub tts_voice: String,
    pub tts_api_base_url: Option<String>,
    pub tts_api_key_env: String,
    pub tts_api_key_present: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceProviderStatus {
    pub provider: String,
    pub lane: String,
    pub kind: String,
    pub api_base_url: String,
    pub api_key_env: String,
    pub api_key_present: bool,
    pub supports_inbound_notes: bool,
    pub supports_voice_catalog: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceProviderCatalog {
    pub stt: Vec<VoiceProviderStatus>,
    pub tts: Vec<VoiceProviderStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceTranscribeRequest {
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
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
pub struct VoiceTranscribeResult {
    pub text: String,
    pub provider: String,
    pub model: String,
    pub language: Option<String>,
    pub duration_secs: Option<f64>,
    pub source: String,
    pub local_path: Option<String>,
    pub max_audio_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceInfoRecord {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub preview_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceVoicesResult {
    pub provider: String,
    pub model: String,
    pub default_voice: String,
    pub voices: Vec<VoiceInfoRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceSynthesizeRequest {
    pub text: String,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub voice: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub output_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceSynthesizeResult {
    pub provider: String,
    pub model: String,
    pub voice: String,
    pub format: String,
    pub text_length: usize,
    pub output_path: String,
    pub bytes: usize,
}

#[derive(Debug, Clone)]
struct ResolvedVoiceProvider {
    provider: String,
    api_key_env: String,
    api_base_url: String,
    lane: String,
    supports_inbound_notes: bool,
    supports_voice_catalog: bool,
    notes: Vec<String>,
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

#[derive(Debug, Deserialize)]
struct DeepgramListenResponse {
    #[serde(default)]
    metadata: Option<DeepgramMetadata>,
    #[serde(default)]
    results: DeepgramResults,
}

#[derive(Debug, Default, Deserialize)]
struct DeepgramMetadata {
    #[serde(default)]
    duration: Option<f64>,
}

#[derive(Debug, Default, Deserialize)]
struct DeepgramResults {
    #[serde(default)]
    channels: Vec<DeepgramChannel>,
}

#[derive(Debug, Default, Deserialize)]
struct DeepgramChannel {
    #[serde(default)]
    alternatives: Vec<DeepgramAlternative>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct DeepgramAlternative {
    #[serde(default)]
    transcript: String,
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    detected_language: Option<String>,
}

#[derive(Debug)]
struct TranscribedAudio {
    text: String,
    language: Option<String>,
    duration_secs: Option<f64>,
    cached_local_path: Option<PathBuf>,
}

#[derive(Debug, Serialize)]
struct OpenAiSpeechRequest<'a> {
    model: &'a str,
    input: &'a str,
    voice: &'a str,
    response_format: &'a str,
}

fn normalize_voice_provider_name(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn default_voice_api_base_url(provider: &str) -> Option<&'static str> {
    match provider {
        "openai" => Some("https://api.openai.com/v1"),
        "openrouter" => Some("https://openrouter.ai/api/v1"),
        "deepgram" => Some("https://api.deepgram.com/v1"),
        _ => None,
    }
}

fn default_voice_api_key_env(config: &AppConfig, provider: &str) -> Option<String> {
    match provider {
        "openai" => Some(
            config
                .providers
                .openai
                .api_key_env
                .clone()
                .unwrap_or_else(|| "OPENAI_API_KEY".to_string()),
        ),
        "openrouter" => Some(
            config
                .providers
                .openrouter
                .api_key_env
                .clone()
                .unwrap_or_else(|| "OPENROUTER_API_KEY".to_string()),
        ),
        "deepgram" => Some("DEEPGRAM_API_KEY".to_string()),
        _ => None,
    }
}

fn resolve_voice_provider_for_stt(
    config: &AppConfig,
    provider_override: Option<&str>,
) -> Result<ResolvedVoiceProvider> {
    let requested = provider_override
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(config.voice.stt.provider.as_str());
    let provider = normalize_voice_provider_name(requested);
    let api_base_url = config
        .voice
        .stt
        .api_base_url
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .or_else(|| default_voice_api_base_url(&provider).map(ToString::to_string))
        .ok_or_else(|| {
            anyhow!(
                "voice STT provider '{}' requires voice.stt.api_base_url for OpenAI-compatible routing",
                provider
            )
        })?;
    let api_key_env = config
        .voice
        .stt
        .api_key_env
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .or_else(|| default_voice_api_key_env(config, &provider))
        .ok_or_else(|| {
            anyhow!(
                "voice STT provider '{}' requires voice.stt.api_key_env or a known provider key binding",
                provider
            )
        })?;

    if provider == "deepgram" {
        return Ok(ResolvedVoiceProvider {
            provider,
            api_key_env,
            api_base_url: api_base_url.trim_end_matches('/').to_string(),
            lane: "deepgram_listen".to_string(),
            supports_inbound_notes: true,
            supports_voice_catalog: false,
            notes: vec![
                "deepgram_rest".to_string(),
                "endpoint must expose /listen".to_string(),
            ],
        });
    }

    let mut notes = vec!["openai_compatible".to_string()];
    if provider != "openai" {
        notes.push("endpoint must expose /audio/transcriptions".to_string());
    }

    Ok(ResolvedVoiceProvider {
        provider,
        api_key_env,
        api_base_url: api_base_url.trim_end_matches('/').to_string(),
        lane: "openai_compatible".to_string(),
        supports_inbound_notes: true,
        supports_voice_catalog: false,
        notes,
    })
}

fn resolve_voice_provider_for_tts(
    config: &AppConfig,
    provider_override: Option<&str>,
) -> Result<ResolvedVoiceProvider> {
    let requested = provider_override
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(config.voice.tts.provider.as_str());
    let provider = normalize_voice_provider_name(requested);
    let api_base_url = config
        .voice
        .tts
        .api_base_url
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .or_else(|| default_voice_api_base_url(&provider).map(ToString::to_string))
        .ok_or_else(|| {
            anyhow!(
                "voice TTS provider '{}' requires voice.tts.api_base_url for OpenAI-compatible routing",
                provider
            )
        })?;
    let api_key_env = config
        .voice
        .tts
        .api_key_env
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .or_else(|| default_voice_api_key_env(config, &provider))
        .ok_or_else(|| {
            anyhow!(
                "voice TTS provider '{}' requires voice.tts.api_key_env or a known provider key binding",
                provider
            )
        })?;

    let mut notes = vec!["openai_compatible".to_string()];
    if provider != "openai" {
        notes.push("endpoint must expose /audio/speech".to_string());
    }

    Ok(ResolvedVoiceProvider {
        provider,
        api_key_env,
        api_base_url: api_base_url.trim_end_matches('/').to_string(),
        lane: "openai_compatible".to_string(),
        supports_inbound_notes: false,
        supports_voice_catalog: true,
        notes,
    })
}

fn provider_status_from_resolved(
    profile: ResolvedVoiceProvider,
    kind: &str,
) -> VoiceProviderStatus {
    VoiceProviderStatus {
        provider: profile.provider,
        lane: profile.lane,
        kind: kind.to_string(),
        api_base_url: profile.api_base_url,
        api_key_env: profile.api_key_env.clone(),
        api_key_present: std::env::var(&profile.api_key_env).is_ok(),
        supports_inbound_notes: profile.supports_inbound_notes,
        supports_voice_catalog: profile.supports_voice_catalog,
        notes: profile.notes,
    }
}

pub fn voice_provider_catalog(config: &AppConfig) -> VoiceProviderCatalog {
    let mut stt = Vec::new();
    let mut tts = Vec::new();

    for provider in ["openai", "openrouter", "deepgram"] {
        if let Ok(profile) = resolve_voice_provider_for_stt(config, Some(provider)) {
            stt.push(provider_status_from_resolved(profile, "stt"));
        }
    }

    for provider in ["openai", "openrouter"] {
        if let Ok(profile) = resolve_voice_provider_for_tts(config, Some(provider)) {
            tts.push(provider_status_from_resolved(profile, "tts"));
        }
    }

    let configured_stt = normalize_voice_provider_name(&config.voice.stt.provider);
    if !["openai", "openrouter", "deepgram"].contains(&configured_stt.as_str()) {
        if let Ok(profile) = resolve_voice_provider_for_stt(config, Some(&configured_stt)) {
            stt.push(provider_status_from_resolved(profile, "stt"));
        }
    }

    let configured_tts = normalize_voice_provider_name(&config.voice.tts.provider);
    if !["openai", "openrouter"].contains(&configured_tts.as_str()) {
        if let Ok(profile) = resolve_voice_provider_for_tts(config, Some(&configured_tts)) {
            tts.push(provider_status_from_resolved(profile, "tts"));
        }
    }

    VoiceProviderCatalog { stt, tts }
}

impl InboundVoiceTranscriber {
    pub fn try_from_config(config: &AppConfig, workspace_root: &Path) -> Result<Option<Self>> {
        if !config.voice.enabled || !config.voice.stt.transcribe_inbound_notes {
            return Ok(None);
        }

        let provider = resolve_voice_provider_for_stt(config, None)?;

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
            provider: provider.provider,
            stt: config.voice.stt.clone(),
            api_key_env: provider.api_key_env,
            api_base_url: provider.api_base_url,
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
                apply_transcription(
                    &mut incoming,
                    &candidate,
                    transcribed,
                    &self.provider,
                    &self.stt.model,
                );
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

        if self.provider == "deepgram" {
            return self
                .transcribe_candidate_deepgram(candidate, &api_key, bytes, cached_local_path)
                .await;
        }

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
            .post(format!("{}/audio/transcriptions", self.api_base_url))
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

    async fn transcribe_candidate_deepgram(
        &self,
        candidate: &AudioCandidate,
        api_key: &str,
        bytes: Vec<u8>,
        cached_local_path: Option<PathBuf>,
    ) -> Result<TranscribedAudio> {
        let mut url = Url::parse(&format!("{}/listen", self.api_base_url))
            .context("failed to build Deepgram listen URL")?;
        {
            let mut pairs = url.query_pairs_mut();
            pairs.append_pair("model", &self.stt.model);
            if self.stt.language != "auto" {
                pairs.append_pair("language", &self.stt.language);
            }
            pairs.append_pair("smart_format", "true");
        }

        let content_type = candidate
            .mime
            .as_deref()
            .unwrap_or("application/octet-stream");

        let response = self
            .client
            .post(url)
            .header(reqwest::header::CONTENT_TYPE, content_type)
            .header(reqwest::header::AUTHORIZATION, format!("Token {api_key}"))
            .body(bytes)
            .send()
            .await
            .context("failed to call Deepgram transcription endpoint")?;
        let response = response
            .error_for_status()
            .context("Deepgram transcription endpoint returned an error")?;
        let parsed: DeepgramListenResponse = response
            .json()
            .await
            .context("failed to parse Deepgram transcription response")?;
        let alternative = parsed
            .results
            .channels
            .first()
            .and_then(|channel| channel.alternatives.first())
            .cloned()
            .unwrap_or_default();

        Ok(TranscribedAudio {
            text: alternative.transcript,
            language: alternative
                .language
                .or(alternative.detected_language)
                .filter(|value| !value.trim().is_empty()),
            duration_secs: parsed.metadata.and_then(|metadata| metadata.duration),
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

pub fn voice_status(config: &AppConfig, workspace_root: &Path) -> VoiceStatus {
    let download_dir = config.voice.stt.download_dir.as_deref().map(|value| {
        resolve_runtime_path(workspace_root, value)
            .to_string_lossy()
            .to_string()
    });
    let stt_provider =
        resolve_voice_provider_for_stt(config, None).unwrap_or_else(|_| ResolvedVoiceProvider {
            provider: normalize_voice_provider_name(&config.voice.stt.provider),
            api_key_env: config.voice.stt.api_key_env.clone().unwrap_or_default(),
            api_base_url: config.voice.stt.api_base_url.clone().unwrap_or_default(),
            lane: "openai_compatible".to_string(),
            supports_inbound_notes: false,
            supports_voice_catalog: false,
            notes: Vec::new(),
        });
    let tts_provider =
        resolve_voice_provider_for_tts(config, None).unwrap_or_else(|_| ResolvedVoiceProvider {
            provider: normalize_voice_provider_name(&config.voice.tts.provider),
            api_key_env: config.voice.tts.api_key_env.clone().unwrap_or_default(),
            api_base_url: config.voice.tts.api_base_url.clone().unwrap_or_default(),
            lane: "openai_compatible".to_string(),
            supports_inbound_notes: false,
            supports_voice_catalog: false,
            notes: Vec::new(),
        });

    VoiceStatus {
        enabled: config.voice.enabled,
        stt_provider: stt_provider.provider,
        stt_model: config.voice.stt.model.clone(),
        stt_language: config.voice.stt.language.clone(),
        transcribe_inbound_notes: config.voice.stt.transcribe_inbound_notes,
        download_dir,
        timeout_secs: config.voice.stt.timeout_secs,
        max_audio_bytes: config.voice.stt.max_audio_bytes,
        api_base_url: (!stt_provider.api_base_url.is_empty()).then_some(stt_provider.api_base_url),
        api_key_env: stt_provider.api_key_env.clone(),
        api_key_present: std::env::var(&stt_provider.api_key_env).is_ok(),
        tts_provider: tts_provider.provider,
        tts_model: config.voice.tts.model.clone(),
        tts_voice: config.voice.tts.voice.clone(),
        tts_api_base_url: (!tts_provider.api_base_url.is_empty())
            .then_some(tts_provider.api_base_url),
        tts_api_key_env: tts_provider.api_key_env.clone(),
        tts_api_key_present: std::env::var(&tts_provider.api_key_env).is_ok(),
    }
}

pub async fn transcribe_with_config(
    config: &AppConfig,
    workspace_root: &Path,
    request: VoiceTranscribeRequest,
) -> Result<VoiceTranscribeResult> {
    let source = match (
        request
            .path
            .as_deref()
            .filter(|value| !value.trim().is_empty()),
        request
            .url
            .as_deref()
            .filter(|value| !value.trim().is_empty()),
    ) {
        (Some(path), None) => AudioSource::Local(PathBuf::from(path)),
        (None, Some(url)) => AudioSource::Remote(url.to_string()),
        (Some(_), Some(_)) => {
            return Err(anyhow!("provide either path or url, not both"));
        }
        (None, None) => {
            return Err(anyhow!("a local path or remote url is required"));
        }
    };

    let mut effective = config.clone();
    if let Some(provider) = request.provider.as_deref() {
        effective.voice.stt.provider = provider.to_string();
    }
    if let Some(model) = request.model.as_deref() {
        effective.voice.stt.model = model.to_string();
    }
    if let Some(language) = request.language.as_deref() {
        effective.voice.stt.language = language.to_string();
    }
    if let Some(prompt) = request.prompt {
        effective.voice.stt.prompt = Some(prompt);
    }
    if let Some(max_audio_bytes) = request.max_audio_bytes {
        effective.voice.stt.max_audio_bytes = max_audio_bytes;
    }
    if let Some(timeout_secs) = request.timeout_secs {
        effective.voice.stt.timeout_secs = timeout_secs;
    }
    effective.voice.enabled = true;
    effective.voice.stt.transcribe_inbound_notes = true;

    let resolved_provider =
        resolve_voice_provider_for_stt(&effective, request.provider.as_deref())?;

    let transcriber = InboundVoiceTranscriber::try_from_config(&effective, workspace_root)?
        .ok_or_else(|| anyhow!("voice transcription is not enabled for the current config"))?;

    let filename = match &source {
        AudioSource::Local(path) => path
            .file_name()
            .and_then(|value| value.to_str())
            .map(ToString::to_string)
            .unwrap_or_else(|| "audio-note.wav".to_string()),
        AudioSource::Remote(url) => url
            .rsplit('/')
            .next()
            .and_then(|value| (!value.trim().is_empty()).then_some(value))
            .map(ToString::to_string)
            .unwrap_or_else(|| "audio-note.wav".to_string()),
    };
    let candidate = AudioCandidate {
        index: 0,
        media_kind: "audio".to_string(),
        source,
        filename: filename.clone(),
        mime: mime_from_filename(&filename),
    };
    let source_label = match &candidate.source {
        AudioSource::Local(path) => path.to_string_lossy().to_string(),
        AudioSource::Remote(url) => url.clone(),
    };
    let transcribed = transcriber.transcribe_candidate(&candidate).await?;

    Ok(VoiceTranscribeResult {
        text: transcribed.text,
        provider: resolved_provider.provider,
        model: transcriber.stt.model.clone(),
        language: transcribed.language,
        duration_secs: transcribed.duration_secs,
        source: source_label,
        local_path: transcribed
            .cached_local_path
            .map(|path| path.to_string_lossy().to_string()),
        max_audio_bytes: transcriber.stt.max_audio_bytes,
    })
}

pub fn list_voices_with_config(
    config: &AppConfig,
    _workspace_root: &Path,
) -> Result<VoiceVoicesResult> {
    let provider = resolve_voice_provider_for_tts(config, None)?;
    let model = config.voice.tts.model.trim().to_string();
    let default_voice = config.voice.tts.voice.trim().to_string();
    let voices = openai_voice_catalog();

    Ok(VoiceVoicesResult {
        provider: provider.provider,
        model,
        default_voice,
        voices,
    })
}

pub async fn synthesize_with_config(
    config: &AppConfig,
    workspace_root: &Path,
    request: VoiceSynthesizeRequest,
) -> Result<VoiceSynthesizeResult> {
    let text = request.text.trim();
    if text.is_empty() {
        return Err(anyhow!("text is required"));
    }

    let mut effective = config.clone();
    if let Some(provider) = request
        .provider
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        effective.voice.tts.provider = provider.to_string();
    }
    if let Some(model) = request
        .model
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        effective.voice.tts.model = model.to_string();
    }
    if let Some(voice) = request
        .voice
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        effective.voice.tts.voice = voice.to_string();
    }

    let format = normalize_speech_format(request.format.as_deref())?;
    let provider = resolve_voice_provider_for_tts(&effective, request.provider.as_deref())?;
    let output_path =
        resolve_speech_output_path(workspace_root, request.output_path.as_deref(), format);

    let bytes = synthesize_openai_compatible(&effective.voice.tts, &provider, text, format).await?;

    if bytes.is_empty() {
        return Err(anyhow!("voice synthesis returned an empty audio payload"));
    }

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .await
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(&output_path, &bytes).await.with_context(|| {
        format!(
            "failed to write synthesized audio {}",
            output_path.display()
        )
    })?;

    Ok(VoiceSynthesizeResult {
        provider: provider.provider,
        model: effective.voice.tts.model.clone(),
        voice: effective.voice.tts.voice.clone(),
        format: format.to_string(),
        text_length: text.chars().count(),
        output_path: output_path.to_string_lossy().to_string(),
        bytes: bytes.len(),
    })
}

fn apply_transcription(
    incoming: &mut IncomingMessage,
    candidate: &AudioCandidate,
    transcribed: TranscribedAudio,
    provider: &str,
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
    metadata.insert("voice_transcription_provider".into(), json!(provider));
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

async fn synthesize_openai_compatible(
    tts: &VoiceTtsRuntimeConfig,
    provider: &ResolvedVoiceProvider,
    text: &str,
    format: &str,
) -> Result<Vec<u8>> {
    let api_key = std::env::var(&provider.api_key_env)
        .with_context(|| format!("{} environment variable not set", provider.api_key_env))?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .context("failed to build voice synthesis HTTP client")?;

    let response = client
        .post(format!("{}/audio/speech", provider.api_base_url))
        .bearer_auth(api_key)
        .json(&OpenAiSpeechRequest {
            model: &tts.model,
            input: text,
            voice: &tts.voice,
            response_format: format,
        })
        .send()
        .await
        .context("failed to call voice synthesis endpoint")?;
    let response = response
        .error_for_status()
        .context("voice synthesis endpoint returned an error")?;
    let bytes = response
        .bytes()
        .await
        .context("failed to read voice synthesis response body")?;
    Ok(bytes.to_vec())
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

fn openai_voice_catalog() -> Vec<VoiceInfoRecord> {
    [
        "alloy", "ash", "ballad", "coral", "echo", "fable", "onyx", "nova", "sage", "shimmer",
    ]
    .into_iter()
    .map(|id| VoiceInfoRecord {
        id: id.to_string(),
        name: id.to_string(),
        language: Some("multilingual".to_string()),
        preview_url: None,
    })
    .collect()
}

fn normalize_speech_format(raw: Option<&str>) -> Result<&'static str> {
    Ok(
        match raw.unwrap_or("mp3").trim().to_ascii_lowercase().as_str() {
            "mp3" => "mp3",
            "wav" => "wav",
            "opus" => "opus",
            "aac" => "aac",
            "flac" => "flac",
            other => return Err(anyhow!("unsupported speech format '{other}'")),
        },
    )
}

fn resolve_speech_output_path(
    workspace_root: &Path,
    raw_output_path: Option<&str>,
    format: &str,
) -> PathBuf {
    if let Some(path) = raw_output_path
        .filter(|value| !value.trim().is_empty())
        .map(|value| resolve_runtime_path(workspace_root, value))
    {
        return path;
    }

    workspace_root
        .join(".claw")
        .join("voice")
        .join("synthesized")
        .join(format!("{}-speech.{format}", Uuid::new_v4()))
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
        speech_bodies: Arc<Mutex<Vec<String>>>,
        deepgram_bodies: Arc<Mutex<Vec<String>>>,
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
            .route("/audio/speech", post(handle_speech))
            .route("/listen", post(handle_deepgram))
            .with_state(state.clone());
        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve");
        });
        (addr, state, handle)
    }

    async fn handle_speech(
        State(state): State<TestServerState>,
        headers: HeaderMap,
        body: Bytes,
    ) -> &'static [u8] {
        assert_eq!(
            headers
                .get(reqwest::header::AUTHORIZATION)
                .and_then(|value| value.to_str().ok()),
            Some("Bearer test-openai-key")
        );
        state
            .speech_bodies
            .lock()
            .await
            .push(String::from_utf8_lossy(body.as_ref()).to_string());
        b"fake-mp3-audio"
    }

    async fn handle_deepgram(
        State(state): State<TestServerState>,
        headers: HeaderMap,
        body: Bytes,
    ) -> &'static str {
        assert_eq!(
            headers
                .get(reqwest::header::AUTHORIZATION)
                .and_then(|value| value.to_str().ok()),
            Some("Token test-deepgram-key")
        );
        state
            .deepgram_bodies
            .lock()
            .await
            .push(String::from_utf8_lossy(body.as_ref()).to_string());
        r#"{"metadata":{"duration":2.5},"results":{"channels":[{"alternatives":[{"transcript":"hello from deepgram","detected_language":"en"}]}]}}"#
    }

    fn test_config(base_url: &str, workspace_root: &Path, api_key_env: &str) -> AppConfig {
        let mut config = AppConfig::default();
        config.voice.enabled = true;
        config.voice.stt.transcribe_inbound_notes = true;
        config.voice.stt.api_base_url = Some(base_url.to_string());
        config.voice.stt.api_key_env = Some(api_key_env.to_string());
        config.voice.stt.download_dir = Some(
            workspace_root
                .join("voice-cache")
                .to_string_lossy()
                .to_string(),
        );
        config.voice.tts.api_base_url = Some(base_url.to_string());
        config.voice.tts.api_key_env = Some(api_key_env.to_string());
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

    #[tokio::test]
    async fn synthesize_writes_audio_artifact_and_records_request() {
        let temp = tempdir().expect("tempdir");
        let (addr, state, server_handle) = spawn_test_server().await;
        let config = test_config(
            &format!("http://{}", addr),
            temp.path(),
            "OPENRUSTCLAW_VOICE_TEST_KEY_THREE",
        );

        unsafe {
            std::env::set_var("OPENRUSTCLAW_VOICE_TEST_KEY_THREE", "test-openai-key");
        }
        let result = synthesize_with_config(
            &config,
            temp.path(),
            VoiceSynthesizeRequest {
                text: "hello from synthesized voice".to_string(),
                provider: None,
                model: Some("tts-1-hd".to_string()),
                voice: Some("nova".to_string()),
                format: Some("mp3".to_string()),
                output_path: None,
            },
        )
        .await
        .expect("synthesize");
        unsafe {
            std::env::remove_var("OPENRUSTCLAW_VOICE_TEST_KEY_THREE");
        }

        assert_eq!(result.provider, "openai");
        assert_eq!(result.model, "tts-1-hd");
        assert_eq!(result.voice, "nova");
        assert_eq!(result.format, "mp3");
        assert!(result.output_path.contains(".claw/voice/synthesized/"));
        assert_eq!(
            fs::read(&result.output_path).await.expect("read output"),
            b"fake-mp3-audio"
        );

        let bodies = state.speech_bodies.lock().await;
        assert_eq!(bodies.len(), 1);
        assert!(bodies[0].contains("\"model\":\"tts-1-hd\""));
        assert!(bodies[0].contains("\"voice\":\"nova\""));
        assert!(bodies[0].contains("\"response_format\":\"mp3\""));

        server_handle.abort();
    }

    #[tokio::test]
    async fn transcribe_supports_deepgram_stt_lane() {
        let temp = tempdir().expect("tempdir");
        let audio_path = temp.path().join("voice.wav");
        fs::write(&audio_path, b"fake wav bytes")
            .await
            .expect("write local audio");

        let (addr, state, server_handle) = spawn_test_server().await;
        let mut config = test_config(
            &format!("http://{}", addr),
            temp.path(),
            "OPENRUSTCLAW_VOICE_TEST_KEY_FOUR",
        );
        config.voice.stt.provider = "deepgram".to_string();
        config.voice.stt.model = "nova-3".to_string();
        config.voice.stt.api_base_url = Some(format!("http://{}", addr));
        config.voice.stt.api_key_env = Some("OPENRUSTCLAW_VOICE_TEST_KEY_FOUR".to_string());

        unsafe {
            std::env::set_var("OPENRUSTCLAW_VOICE_TEST_KEY_FOUR", "test-deepgram-key");
        }
        let result = transcribe_with_config(
            &config,
            temp.path(),
            VoiceTranscribeRequest {
                path: Some(audio_path.to_string_lossy().to_string()),
                url: None,
                provider: Some("deepgram".to_string()),
                model: None,
                language: None,
                prompt: None,
                max_audio_bytes: None,
                timeout_secs: None,
            },
        )
        .await
        .expect("deepgram transcribe");
        unsafe {
            std::env::remove_var("OPENRUSTCLAW_VOICE_TEST_KEY_FOUR");
        }

        assert_eq!(result.provider, "deepgram");
        assert_eq!(result.model, "nova-3");
        assert_eq!(result.text, "hello from deepgram");
        assert_eq!(result.language.as_deref(), Some("en"));

        let bodies = state.deepgram_bodies.lock().await;
        assert_eq!(bodies.len(), 1);

        server_handle.abort();
    }

    #[test]
    fn list_voices_returns_known_openai_voice_catalog() {
        let temp = tempdir().expect("tempdir");
        let config = AppConfig::default();
        let result = list_voices_with_config(&config, temp.path()).expect("list voices");
        assert_eq!(result.provider, "openai");
        assert!(result.voices.iter().any(|voice| voice.id == "alloy"));
        assert!(result.voices.iter().any(|voice| voice.id == "nova"));
    }

    #[test]
    fn provider_catalog_reports_openai_and_openrouter_lanes() {
        let mut config = AppConfig::default();
        config.providers.openai.api_key_env = Some("OPENAI_TEST_KEY".to_string());
        config.providers.openrouter.api_key_env = Some("OPENROUTER_TEST_KEY".to_string());
        config.voice.stt.provider = "deepgram".to_string();
        config.voice.stt.api_key_env = Some("DEEPGRAM_TEST_KEY".to_string());
        let catalog = voice_provider_catalog(&config);

        assert!(catalog.stt.iter().any(|entry| entry.provider == "openai"));
        assert!(
            catalog
                .stt
                .iter()
                .any(|entry| entry.provider == "openrouter")
        );
        assert!(catalog.stt.iter().any(|entry| entry.provider == "deepgram"));
        assert!(catalog.tts.iter().any(|entry| entry.provider == "openai"));
        assert!(
            catalog
                .tts
                .iter()
                .any(|entry| entry.provider == "openrouter")
        );
        assert!(
            catalog
                .tts
                .iter()
                .find(|entry| entry.provider == "openrouter")
                .expect("openrouter tts")
                .notes
                .iter()
                .any(|note| note.contains("/audio/speech"))
        );
    }
}
