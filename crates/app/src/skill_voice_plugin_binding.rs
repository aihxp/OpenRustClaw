use openrustclaw_core::error::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillVoicePluginBindingRequest {
    pub plugin_id: String,
    pub skill_name: String,
    #[serde(default)]
    pub declared_voice_call_plugins: Vec<String>,
    pub blocked: bool,
    #[serde(default)]
    pub service: Option<String>,
    #[serde(default)]
    pub component: Option<String>,
    #[serde(default)]
    pub greeting_text: Option<String>,
    #[serde(default)]
    pub default_voice: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillVoicePluginBinding {
    pub plugin_id: String,
    pub skill_name: String,
    #[serde(default)]
    pub service: Option<String>,
    #[serde(default)]
    pub component: Option<String>,
    #[serde(default)]
    pub greeting_text: Option<String>,
    #[serde(default)]
    pub default_voice: Option<String>,
    pub configured_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillVoicePluginBindingReport {
    pub status: String,
    pub binding: SkillVoicePluginBinding,
}

pub struct SkillVoicePluginBindingService;

impl Default for SkillVoicePluginBindingService {
    fn default() -> Self {
        Self
    }
}

impl SkillVoicePluginBindingService {
    pub fn new() -> Self {
        Self
    }

    pub fn bind(
        &self,
        request: SkillVoicePluginBindingRequest,
        existing_configured_at: Option<String>,
        now: String,
    ) -> Result<SkillVoicePluginBindingReport> {
        if request.blocked {
            return Err(Error::Internal(format!(
                "Compiled skill '{}' is blocked and cannot be bound as a voice plugin",
                request.skill_name
            )));
        }

        if !request.declared_voice_call_plugins.is_empty()
            && !request
                .declared_voice_call_plugins
                .iter()
                .any(|plugin| plugin == &request.plugin_id)
        {
            return Err(Error::Internal(format!(
                "Compiled skill '{}' declares voice call plugins [{}], not '{}'",
                request.skill_name,
                request.declared_voice_call_plugins.join(", "),
                request.plugin_id
            )));
        }

        let configured_at = existing_configured_at.unwrap_or_else(|| now.clone());
        Ok(SkillVoicePluginBindingReport {
            status: "ok".to_string(),
            binding: SkillVoicePluginBinding {
                plugin_id: request.plugin_id,
                skill_name: request.skill_name,
                service: request.service,
                component: request.component,
                greeting_text: request.greeting_text,
                default_voice: request.default_voice,
                configured_at,
                updated_at: now,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bind_voice_plugin_rejects_blocked_skill() {
        let error = SkillVoicePluginBindingService::new()
            .bind(
                SkillVoicePluginBindingRequest {
                    plugin_id: "support-line".to_string(),
                    skill_name: "voice-skill".to_string(),
                    declared_voice_call_plugins: vec![],
                    blocked: true,
                    service: None,
                    component: None,
                    greeting_text: None,
                    default_voice: None,
                },
                None,
                "2026-03-28T00:00:00Z".to_string(),
            )
            .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("cannot be bound as a voice plugin")
        );
    }

    #[test]
    fn bind_voice_plugin_rejects_undeclared_plugin_id() {
        let error = SkillVoicePluginBindingService::new()
            .bind(
                SkillVoicePluginBindingRequest {
                    plugin_id: "support-line".to_string(),
                    skill_name: "voice-skill".to_string(),
                    declared_voice_call_plugins: vec!["billing-line".to_string()],
                    blocked: false,
                    service: None,
                    component: None,
                    greeting_text: None,
                    default_voice: None,
                },
                None,
                "2026-03-28T00:00:00Z".to_string(),
            )
            .unwrap_err();

        assert!(error.to_string().contains("declares voice call plugins"));
    }

    #[test]
    fn bind_voice_plugin_preserves_existing_configured_at() -> Result<()> {
        let report = SkillVoicePluginBindingService::new().bind(
            SkillVoicePluginBindingRequest {
                plugin_id: "support-line".to_string(),
                skill_name: "voice-skill".to_string(),
                declared_voice_call_plugins: vec!["support-line".to_string()],
                blocked: false,
                service: Some("voice".to_string()),
                component: Some("call".to_string()),
                greeting_text: Some("hello".to_string()),
                default_voice: Some("nova".to_string()),
            },
            Some("2026-03-27T10:00:00Z".to_string()),
            "2026-03-28T12:00:00Z".to_string(),
        )?;

        assert_eq!(report.status, "ok");
        assert_eq!(report.binding.plugin_id, "support-line");
        assert_eq!(report.binding.skill_name, "voice-skill");
        assert_eq!(
            report.binding.configured_at,
            "2026-03-27T10:00:00Z".to_string()
        );
        assert_eq!(
            report.binding.updated_at,
            "2026-03-28T12:00:00Z".to_string()
        );
        assert_eq!(report.binding.service.as_deref(), Some("voice"));
        assert_eq!(report.binding.component.as_deref(), Some("call"));
        Ok(())
    }
}
