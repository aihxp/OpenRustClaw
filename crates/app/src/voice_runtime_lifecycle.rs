use chrono::{DateTime, Utc};
use openrustclaw_core::config::AppConfig;
use openrustclaw_core::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::env;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceProviderCatalog {
    pub stt: Vec<VoiceProviderStatus>,
    pub tts: Vec<VoiceProviderStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceSynthesizeRequest {
    pub text: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub voice: Option<String>,
    pub format: Option<String>,
    pub output_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceSynthesizeResult {
    pub provider: String,
    pub model: String,
    pub voice: String,
    pub format: String,
    pub text_length: usize,
    pub output_path: String,
    pub bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceSessionTurn {
    pub role: String,
    pub text: String,
    pub created_at: String,
    pub synthesized_output_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceSessionRecord {
    pub id: String,
    pub status: String,
    pub live_state: String,
    pub stt_provider: String,
    pub tts_provider: String,
    pub tts_voice: String,
    pub assistant_prompt: Option<String>,
    pub end_reason: Option<String>,
    pub created_at: String,
    pub last_activity_at: String,
    pub closed_at: Option<String>,
    pub reconnect_count: usize,
    pub last_reconnected_at: Option<String>,
    pub pause_count: usize,
    pub interrupted_count: usize,
    pub last_paused_at: Option<String>,
    pub last_resumed_at: Option<String>,
    pub last_interrupted_at: Option<String>,
    pub turns: Vec<VoiceSessionTurn>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceSessionList {
    pub sessions: Vec<VoiceSessionRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceSessionHealthRecord {
    pub id: String,
    pub status: String,
    pub last_activity_at: String,
    pub closed_at: Option<String>,
    pub turn_count: usize,
    pub idle_secs: u64,
    pub stale: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceSessionHealthSummary {
    pub total_sessions: usize,
    pub active_sessions: usize,
    pub ended_sessions: usize,
    pub stale_after_secs: u64,
    pub stale_sessions: usize,
    pub oldest_active_idle_secs: Option<u64>,
    pub sessions: Vec<VoiceSessionHealthRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceSessionStartRequest {
    pub session_id: Option<String>,
    pub assistant_prompt: Option<String>,
    pub voice: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceSessionAppendRequest {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceSessionRespondRequest {
    pub text: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub voice: Option<String>,
    pub format: Option<String>,
    pub output_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceSessionEndRequest {
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceSessionRespondResult {
    pub session: VoiceSessionRecord,
    pub synthesis: VoiceSynthesizeResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct VoiceSessionReconnectRequest {
    pub assistant_prompt: Option<String>,
    pub voice: Option<String>,
    pub greeting: Option<String>,
    pub format: Option<String>,
    pub output_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceSessionReconnectResult {
    pub session: VoiceSessionRecord,
    pub synthesis: Option<VoiceSynthesizeResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct VoiceSessionControlRequest {
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct VoiceSessionHealthRequest {
    pub stale_after_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct VoiceSessionReapRequest {
    pub stale_after_secs: Option<u64>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceSessionReapResult {
    pub stale_after_secs: u64,
    pub reason: String,
    pub reaped_sessions: Vec<VoiceSessionRecord>,
    pub health: VoiceSessionHealthSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct VoicePrewarmRequest {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub voice: Option<String>,
    pub format: Option<String>,
    pub greeting: Option<String>,
    pub output_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedVoiceProvider {
    pub provider: String,
    pub api_key_env: String,
    pub api_base_url: String,
    pub lane: String,
    pub supports_inbound_notes: bool,
    pub supports_voice_catalog: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct VoiceRuntimeLifecycleService;

impl VoiceRuntimeLifecycleService {
    pub fn new() -> Self {
        Self
    }

    pub fn provider_catalog(&self, config: &AppConfig) -> VoiceProviderCatalog {
        let mut stt = Vec::new();
        let mut tts = Vec::new();

        for provider in ["openai", "openrouter", "deepgram"] {
            if let Ok(profile) = self.resolve_voice_provider_for_stt(config, Some(provider)) {
                stt.push(self.provider_status_from_resolved(profile, "stt"));
            }
        }

        for provider in ["openai", "openrouter"] {
            if let Ok(profile) = self.resolve_voice_provider_for_tts(config, Some(provider)) {
                tts.push(self.provider_status_from_resolved(profile, "tts"));
            }
        }

        let configured_stt = self.normalize_voice_provider_name(&config.voice.stt.provider);
        if !["openai", "openrouter", "deepgram"].contains(&configured_stt.as_str())
            && let Ok(profile) = self.resolve_voice_provider_for_stt(config, Some(&configured_stt))
        {
            stt.push(self.provider_status_from_resolved(profile, "stt"));
        }

        let configured_tts = self.normalize_voice_provider_name(&config.voice.tts.provider);
        if !["openai", "openrouter"].contains(&configured_tts.as_str())
            && let Ok(profile) = self.resolve_voice_provider_for_tts(config, Some(&configured_tts))
        {
            tts.push(self.provider_status_from_resolved(profile, "tts"));
        }

        VoiceProviderCatalog { stt, tts }
    }

    pub fn resolve_voice_provider_for_stt(
        &self,
        config: &AppConfig,
        provider_override: Option<&str>,
    ) -> Result<ResolvedVoiceProvider> {
        let requested = provider_override
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(config.voice.stt.provider.as_str());
        let provider = self.normalize_voice_provider_name(requested);
        let api_base_url = config
            .voice
            .stt
            .api_base_url
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
            .or_else(|| self.default_voice_api_base_url(&provider).map(ToString::to_string))
            .ok_or_else(|| {
                Error::Internal(format!(
                    "voice STT provider '{}' requires voice.stt.api_base_url for OpenAI-compatible routing",
                    provider
                ))
            })?;
        let api_key_env = config
            .voice
            .stt
            .api_key_env
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
            .or_else(|| self.default_voice_api_key_env(config, &provider))
            .ok_or_else(|| {
                Error::Internal(format!(
                    "voice STT provider '{}' requires voice.stt.api_key_env or a known provider key binding",
                    provider
                ))
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

    pub fn resolve_voice_provider_for_tts(
        &self,
        config: &AppConfig,
        provider_override: Option<&str>,
    ) -> Result<ResolvedVoiceProvider> {
        let requested = provider_override
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(config.voice.tts.provider.as_str());
        let provider = self.normalize_voice_provider_name(requested);
        let api_base_url = config
            .voice
            .tts
            .api_base_url
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
            .or_else(|| self.default_voice_api_base_url(&provider).map(ToString::to_string))
            .ok_or_else(|| {
                Error::Internal(format!(
                    "voice TTS provider '{}' requires voice.tts.api_base_url for OpenAI-compatible routing",
                    provider
                ))
            })?;
        let api_key_env = config
            .voice
            .tts
            .api_key_env
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
            .or_else(|| self.default_voice_api_key_env(config, &provider))
            .ok_or_else(|| {
                Error::Internal(format!(
                    "voice TTS provider '{}' requires voice.tts.api_key_env or a known provider key binding",
                    provider
                ))
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

    pub fn start_session(
        &self,
        config: &AppConfig,
        request: VoiceSessionStartRequest,
        now: String,
    ) -> Result<VoiceSessionRecord> {
        let session_id = request
            .session_id
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let stt_provider = self.resolve_voice_provider_for_stt(config, None)?.provider;
        let tts_provider = self.resolve_voice_provider_for_tts(config, None)?.provider;
        let tts_voice = request
            .voice
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| config.voice.tts.voice.clone());

        Ok(VoiceSessionRecord {
            id: session_id,
            status: "active".to_string(),
            live_state: "listening".to_string(),
            stt_provider,
            tts_provider,
            tts_voice,
            assistant_prompt: request.assistant_prompt,
            end_reason: None,
            created_at: now.clone(),
            last_activity_at: now,
            closed_at: None,
            reconnect_count: 0,
            last_reconnected_at: None,
            pause_count: 0,
            interrupted_count: 0,
            last_paused_at: None,
            last_resumed_at: None,
            last_interrupted_at: None,
            turns: Vec::new(),
        })
    }

    pub fn append_user(
        &self,
        mut session: VoiceSessionRecord,
        request: VoiceSessionAppendRequest,
        now: String,
    ) -> Result<VoiceSessionRecord> {
        self.ensure_voice_session_accepts_turns(&session)?;
        let text = request.text.trim();
        if text.is_empty() {
            return Err(Error::Internal("text is required".to_string()));
        }
        session.turns.push(VoiceSessionTurn {
            role: "user".to_string(),
            text: text.to_string(),
            created_at: now.clone(),
            synthesized_output_path: None,
        });
        session.live_state = "listening".to_string();
        session.last_activity_at = now;
        Ok(session)
    }

    pub fn respond(
        &self,
        mut session: VoiceSessionRecord,
        request: VoiceSessionRespondRequest,
        synthesis: VoiceSynthesizeResult,
        now: String,
    ) -> Result<VoiceSessionRespondResult> {
        self.ensure_voice_session_accepts_turns(&session)?;
        let text = request.text.trim();
        if text.is_empty() {
            return Err(Error::Internal("text is required".to_string()));
        }
        if let Some(voice) = request.voice.filter(|value| !value.trim().is_empty()) {
            session.tts_voice = voice;
        }
        session.turns.push(VoiceSessionTurn {
            role: "assistant".to_string(),
            text: text.to_string(),
            created_at: now.clone(),
            synthesized_output_path: Some(synthesis.output_path.clone()),
        });
        session.live_state = "speaking".to_string();
        session.last_activity_at = now;
        Ok(VoiceSessionRespondResult { session, synthesis })
    }

    pub fn end(
        &self,
        mut session: VoiceSessionRecord,
        request: VoiceSessionEndRequest,
        now: String,
    ) -> Result<VoiceSessionRecord> {
        self.ensure_voice_session_active(&session)?;
        session.status = "ended".to_string();
        session.live_state = "ended".to_string();
        session.end_reason = request.reason;
        session.last_activity_at = now.clone();
        session.closed_at = Some(now);
        Ok(session)
    }

    pub fn reconnect(
        &self,
        mut session: VoiceSessionRecord,
        request: VoiceSessionReconnectRequest,
        synthesis: Option<VoiceSynthesizeResult>,
        now: String,
    ) -> Result<VoiceSessionReconnectResult> {
        self.ensure_voice_session_accepts_turns(&session)?;
        if let Some(assistant_prompt) = request
            .assistant_prompt
            .filter(|value| !value.trim().is_empty())
        {
            session.assistant_prompt = Some(assistant_prompt);
        }
        if let Some(voice) = request.voice.filter(|value| !value.trim().is_empty()) {
            session.tts_voice = voice;
        }

        session.reconnect_count += 1;
        session.last_reconnected_at = Some(now.clone());
        session.last_activity_at = now.clone();
        session.live_state = "listening".to_string();

        if let (Some(greeting), Some(synthesis)) = (
            request.greeting.filter(|value| !value.trim().is_empty()),
            synthesis.as_ref(),
        ) {
            session.turns.push(VoiceSessionTurn {
                role: "assistant".to_string(),
                text: greeting,
                created_at: now,
                synthesized_output_path: Some(synthesis.output_path.clone()),
            });
        }

        Ok(VoiceSessionReconnectResult { session, synthesis })
    }

    pub fn pause(
        &self,
        mut session: VoiceSessionRecord,
        now: String,
    ) -> Result<VoiceSessionRecord> {
        self.ensure_voice_session_active(&session)?;
        if session.live_state == "paused" {
            return Err(Error::Internal(format!(
                "voice session '{}' is already paused",
                session.id
            )));
        }
        session.live_state = "paused".to_string();
        session.pause_count += 1;
        session.last_paused_at = Some(now.clone());
        session.last_activity_at = now;
        Ok(session)
    }

    pub fn resume(
        &self,
        mut session: VoiceSessionRecord,
        now: String,
    ) -> Result<VoiceSessionRecord> {
        self.ensure_voice_session_active(&session)?;
        if !matches!(session.live_state.as_str(), "paused" | "interrupted") {
            return Err(Error::Internal(format!(
                "voice session '{}' is not paused or interrupted ({})",
                session.id, session.live_state
            )));
        }
        session.live_state = "listening".to_string();
        session.last_resumed_at = Some(now.clone());
        session.last_activity_at = now;
        Ok(session)
    }

    pub fn interrupt(
        &self,
        mut session: VoiceSessionRecord,
        now: String,
    ) -> Result<VoiceSessionRecord> {
        self.ensure_voice_session_active(&session)?;
        session.live_state = "interrupted".to_string();
        session.interrupted_count += 1;
        session.last_interrupted_at = Some(now.clone());
        session.last_activity_at = now;
        Ok(session)
    }

    pub fn summarize_sessions(
        &self,
        sessions: Vec<VoiceSessionRecord>,
        stale_after_secs: u64,
        now: DateTime<Utc>,
    ) -> VoiceSessionHealthSummary {
        let total_sessions = sessions.len();
        let active_sessions = sessions
            .iter()
            .filter(|entry| entry.status == "active")
            .count();
        let ended_sessions = sessions
            .iter()
            .filter(|entry| entry.status == "ended")
            .count();
        let mut stale_sessions = 0usize;
        let mut oldest_active_idle_secs = None;
        let mut rows = Vec::new();

        for session in sessions {
            let idle_secs = self.voice_session_idle_secs(&session, now);
            let stale = session.status == "active" && idle_secs >= stale_after_secs;
            if stale {
                stale_sessions += 1;
            }
            if session.status == "active" {
                oldest_active_idle_secs = Some(oldest_active_idle_secs.unwrap_or(0).max(idle_secs));
            }
            rows.push(VoiceSessionHealthRecord {
                id: session.id,
                status: session.status,
                last_activity_at: session.last_activity_at,
                closed_at: session.closed_at,
                turn_count: session.turns.len(),
                idle_secs,
                stale,
            });
        }

        rows.sort_by(|left, right| right.last_activity_at.cmp(&left.last_activity_at));
        VoiceSessionHealthSummary {
            total_sessions,
            active_sessions,
            ended_sessions,
            stale_after_secs,
            stale_sessions,
            oldest_active_idle_secs,
            sessions: rows,
        }
    }

    pub fn reap_sessions(
        &self,
        sessions: Vec<VoiceSessionRecord>,
        stale_after_secs: u64,
        reason: &str,
        now: DateTime<Utc>,
    ) -> Vec<VoiceSessionRecord> {
        let now = now.to_rfc3339();
        let mut reaped = Vec::new();
        for mut session in sessions {
            if session.status != "active" {
                continue;
            }
            let idle_secs = self.voice_session_idle_secs(&session, Utc::now());
            if idle_secs < stale_after_secs {
                continue;
            }
            session.status = "ended".to_string();
            session.live_state = "ended".to_string();
            session.end_reason = Some(reason.to_string());
            session.last_activity_at = now.clone();
            session.closed_at = Some(now.clone());
            reaped.push(session);
        }
        reaped
    }

    pub fn prewarm_synthesis_request(
        &self,
        request: VoicePrewarmRequest,
    ) -> VoiceSynthesizeRequest {
        let greeting = request
            .greeting
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("OpenRustClaw voice runtime warmup.");
        VoiceSynthesizeRequest {
            text: greeting.to_string(),
            provider: request.provider,
            model: request.model,
            voice: request.voice,
            format: request.format,
            output_path: request.output_path,
        }
    }

    pub fn resolved_stale_after_secs(&self, value: Option<u64>) -> u64 {
        value.unwrap_or(300).max(30)
    }

    pub fn voice_session_idle_secs(&self, session: &VoiceSessionRecord, now: DateTime<Utc>) -> u64 {
        self.parse_rfc3339_utc(&session.last_activity_at)
            .map(|last| now.signed_duration_since(last).num_seconds().max(0) as u64)
            .unwrap_or(0)
    }

    fn ensure_voice_session_active(&self, session: &VoiceSessionRecord) -> Result<()> {
        if session.status != "active" {
            return Err(Error::Internal(format!(
                "voice session '{}' is not active ({})",
                session.id, session.status
            )));
        }
        Ok(())
    }

    fn ensure_voice_session_accepts_turns(&self, session: &VoiceSessionRecord) -> Result<()> {
        self.ensure_voice_session_active(session)?;
        if session.live_state == "paused" {
            return Err(Error::Internal(format!(
                "voice session '{}' is paused",
                session.id
            )));
        }
        Ok(())
    }

    fn normalize_voice_provider_name(&self, value: &str) -> String {
        value.trim().to_ascii_lowercase()
    }

    fn default_voice_api_base_url(&self, provider: &str) -> Option<&'static str> {
        match provider {
            "openai" => Some("https://api.openai.com/v1"),
            "openrouter" => Some("https://openrouter.ai/api/v1"),
            "deepgram" => Some("https://api.deepgram.com/v1"),
            _ => None,
        }
    }

    fn default_voice_api_key_env(&self, config: &AppConfig, provider: &str) -> Option<String> {
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

    fn provider_status_from_resolved(
        &self,
        profile: ResolvedVoiceProvider,
        kind: &str,
    ) -> VoiceProviderStatus {
        VoiceProviderStatus {
            provider: profile.provider,
            lane: profile.lane,
            kind: kind.to_string(),
            api_base_url: profile.api_base_url,
            api_key_env: profile.api_key_env.clone(),
            api_key_present: env::var(&profile.api_key_env).is_ok(),
            supports_inbound_notes: profile.supports_inbound_notes,
            supports_voice_catalog: profile.supports_voice_catalog,
            notes: profile.notes,
        }
    }

    fn parse_rfc3339_utc(&self, value: &str) -> Option<DateTime<Utc>> {
        DateTime::parse_from_rfc3339(value)
            .ok()
            .map(|parsed| parsed.with_timezone(&Utc))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> AppConfig {
        AppConfig::default()
    }

    #[test]
    fn provider_catalog_surfaces_known_lanes() {
        let catalog = VoiceRuntimeLifecycleService::new().provider_catalog(&test_config());
        assert!(catalog.stt.iter().any(|entry| entry.provider == "openai"));
        assert!(catalog.stt.iter().any(|entry| entry.provider == "deepgram"));
        assert!(
            catalog
                .tts
                .iter()
                .any(|entry| entry.provider == "openrouter")
        );
    }

    #[test]
    fn lifecycle_transitions_update_voice_session_state() -> Result<()> {
        let service = VoiceRuntimeLifecycleService::new();
        let config = test_config();
        let session = service.start_session(
            &config,
            VoiceSessionStartRequest {
                session_id: Some("voice-1".to_string()),
                assistant_prompt: None,
                voice: None,
            },
            "2026-03-28T20:00:00Z".to_string(),
        )?;
        let session = service.append_user(
            session,
            VoiceSessionAppendRequest {
                text: "hello".to_string(),
            },
            "2026-03-28T20:00:01Z".to_string(),
        )?;
        let response = service.respond(
            session,
            VoiceSessionRespondRequest {
                text: "hi".to_string(),
                provider: None,
                model: None,
                voice: Some("alloy".to_string()),
                format: None,
                output_path: None,
            },
            VoiceSynthesizeResult {
                provider: "openai".to_string(),
                model: "gpt-4o-mini-tts".to_string(),
                voice: "alloy".to_string(),
                format: "mp3".to_string(),
                text_length: 2,
                output_path: "voice/out.mp3".to_string(),
                bytes: 12,
            },
            "2026-03-28T20:00:02Z".to_string(),
        )?;
        let paused = service.pause(response.session, "2026-03-28T20:00:03Z".to_string())?;
        let resumed = service.resume(paused, "2026-03-28T20:00:04Z".to_string())?;
        let ended = service.end(
            resumed,
            VoiceSessionEndRequest {
                reason: Some("done".to_string()),
            },
            "2026-03-28T20:00:05Z".to_string(),
        )?;
        assert_eq!(ended.status, "ended");
        assert_eq!(ended.live_state, "ended");
        assert_eq!(ended.turns.len(), 2);
        Ok(())
    }
}
