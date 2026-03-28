use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeAppliedSnapshot {
    pub captured_at: String,
    pub config_path: String,
    pub default_provider: String,
    pub enabled_channels: Vec<String>,
    pub provider_fingerprint: String,
    pub gateway_fingerprint: String,
    pub security_fingerprint: String,
    pub scheduler_fingerprint: String,
    pub sidecar_fingerprint: String,
    pub channel_fingerprint: String,
    pub channel_route_fingerprint: String,
    pub artifact_fingerprint: String,
    pub artifact_paths: Vec<String>,
    pub persona_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeReloadPlan {
    pub generated_at: String,
    pub config_path: String,
    pub applied_snapshot_at: Option<String>,
    pub status: String,
    pub live_reload_ready: bool,
    pub restart_required: bool,
    pub live_reload_changes: Vec<String>,
    pub provider_changes: Vec<String>,
    pub artifact_changes: Vec<String>,
    pub restart_required_reasons: Vec<String>,
    pub changed_artifacts: Vec<String>,
}

pub struct RuntimeReloadPlanningService;

impl Default for RuntimeReloadPlanningService {
    fn default() -> Self {
        Self
    }
}

impl RuntimeReloadPlanningService {
    pub fn new() -> Self {
        Self
    }

    pub fn build_plan(
        &self,
        current: RuntimeAppliedSnapshot,
        applied: Option<RuntimeAppliedSnapshot>,
        generated_at: String,
    ) -> RuntimeReloadPlan {
        let mut live_reload_changes = Vec::new();
        let mut provider_changes = Vec::new();
        let mut artifact_changes = Vec::new();
        let mut restart_required_reasons = Vec::new();

        if let Some(previous) = &applied {
            if previous.provider_fingerprint != current.provider_fingerprint {
                provider_changes.push(format!(
                    "Provider/model routing changed: {} -> {}",
                    previous.default_provider, current.default_provider
                ));
                live_reload_changes.push(
                    "Provider and model routing can rebind live through the shipped runtime reload path."
                        .to_string(),
                );
            }

            if previous.artifact_fingerprint != current.artifact_fingerprint {
                artifact_changes.push(
                    "Workspace persona/instruction artifacts changed and will be read on the next turn."
                        .to_string(),
                );
                live_reload_changes.push(
                    "Prompt artifact changes are live-safe because the agent runtime resolves workspace artifacts per request."
                        .to_string(),
                );
            }

            if previous.gateway_fingerprint != current.gateway_fingerprint {
                restart_required_reasons.push(
                    "Gateway host/port settings changed and require a process restart.".to_string(),
                );
            }
            if previous.security_fingerprint != current.security_fingerprint {
                restart_required_reasons.push(
                    "Gateway auth/origin policy changed and requires a process restart."
                        .to_string(),
                );
            }
            if previous.scheduler_fingerprint != current.scheduler_fingerprint {
                restart_required_reasons
                    .push("Scheduler timing changed and requires worker restart.".to_string());
            }
            if previous.sidecar_fingerprint != current.sidecar_fingerprint {
                restart_required_reasons.push(
                    "Sidecar launch settings changed and require process restart.".to_string(),
                );
            }
            if previous.enabled_channels != current.enabled_channels {
                restart_required_reasons.push(
                    "Enabled channel set changed and requires channel transport restart."
                        .to_string(),
                );
            } else if previous.channel_route_fingerprint != current.channel_route_fingerprint {
                restart_required_reasons.push(
                    "Webhook or transport-routing paths changed and require process restart."
                        .to_string(),
                );
            } else if previous.channel_fingerprint != current.channel_fingerprint {
                restart_required_reasons.push(
                    "Running channel transport settings changed; restart is required to reconnect safely."
                        .to_string(),
                );
            }
        }

        let changed_artifacts = applied
            .as_ref()
            .map(|previous| {
                current
                    .artifact_paths
                    .iter()
                    .filter(|path| !previous.artifact_paths.contains(*path))
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let restart_required = !restart_required_reasons.is_empty();
        let live_reload_ready = !restart_required
            && (!live_reload_changes.is_empty()
                || applied
                    .as_ref()
                    .map(|previous| previous.provider_fingerprint != current.provider_fingerprint)
                    .unwrap_or(false));
        let status = if restart_required {
            "restart_required"
        } else if live_reload_ready {
            "live_reload_ready"
        } else {
            "up_to_date"
        };

        RuntimeReloadPlan {
            generated_at,
            config_path: current.config_path.clone(),
            applied_snapshot_at: applied.map(|snapshot| snapshot.captured_at),
            status: status.to_string(),
            live_reload_ready,
            restart_required,
            live_reload_changes,
            provider_changes,
            artifact_changes,
            restart_required_reasons,
            changed_artifacts,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> RuntimeAppliedSnapshot {
        RuntimeAppliedSnapshot {
            captured_at: "2026-03-28T10:00:00Z".to_string(),
            config_path: "config/default.toml".to_string(),
            default_provider: "anthropic".to_string(),
            enabled_channels: vec!["telegram".to_string()],
            provider_fingerprint: "provider-a".to_string(),
            gateway_fingerprint: "gateway-a".to_string(),
            security_fingerprint: "security-a".to_string(),
            scheduler_fingerprint: "scheduler-a".to_string(),
            sidecar_fingerprint: "sidecar-a".to_string(),
            channel_fingerprint: "channel-a".to_string(),
            channel_route_fingerprint: "route-a".to_string(),
            artifact_fingerprint: "artifact-a".to_string(),
            artifact_paths: vec![".claw/artifacts/persona.md".to_string()],
            persona_paths: vec![".claw/artifacts/persona.md".to_string()],
        }
    }

    #[test]
    fn runtime_reload_plan_marks_provider_delta_live_reload_ready() {
        let mut current = snapshot();
        current.default_provider = "openai".to_string();
        current.provider_fingerprint = "provider-b".to_string();

        let plan = RuntimeReloadPlanningService::new().build_plan(
            current,
            Some(snapshot()),
            "2026-03-28T12:00:00Z".to_string(),
        );

        assert_eq!(plan.status, "live_reload_ready");
        assert!(plan.live_reload_ready);
        assert!(!plan.restart_required);
        assert_eq!(plan.provider_changes.len(), 1);
    }

    #[test]
    fn runtime_reload_plan_marks_gateway_delta_restart_required() {
        let mut current = snapshot();
        current.gateway_fingerprint = "gateway-b".to_string();

        let plan = RuntimeReloadPlanningService::new().build_plan(
            current,
            Some(snapshot()),
            "2026-03-28T12:00:00Z".to_string(),
        );

        assert_eq!(plan.status, "restart_required");
        assert!(plan.restart_required);
        assert!(!plan.live_reload_ready);
        assert_eq!(plan.restart_required_reasons.len(), 1);
    }
}
