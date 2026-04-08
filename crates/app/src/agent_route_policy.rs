use crate::agent_fabric_registry::FabricRouteSignal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct DelegatedRouteRequest {
    #[serde(default)]
    pub preferred_backend_id: Option<String>,
    #[serde(default)]
    pub preferred_provider_id: Option<String>,
    #[serde(default)]
    pub preferred_host_id: Option<String>,
    #[serde(default)]
    pub operator_id: Option<String>,
    #[serde(default)]
    pub task_summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DelegatedRouteCandidateReceipt {
    pub host_scope: String,
    pub host_id: String,
    pub backend_id: String,
    #[serde(default)]
    pub provider_id: Option<String>,
    pub routeable: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DelegatedRouteDecision {
    pub receipt_id: String,
    pub decided_at: String,
    pub status: String,
    #[serde(default)]
    pub requested_backend_id: Option<String>,
    #[serde(default)]
    pub requested_provider_id: Option<String>,
    #[serde(default)]
    pub requested_host_id: Option<String>,
    #[serde(default)]
    pub selected_host_scope: Option<String>,
    #[serde(default)]
    pub selected_host_id: Option<String>,
    #[serde(default)]
    pub selected_host_label: Option<String>,
    #[serde(default)]
    pub selected_backend_id: Option<String>,
    #[serde(default)]
    pub selected_provider_id: Option<String>,
    pub reason: String,
    #[serde(default)]
    pub recovery_hints: Vec<String>,
    #[serde(default)]
    pub candidates: Vec<DelegatedRouteCandidateReceipt>,
    #[serde(default)]
    pub operator_id: Option<String>,
    #[serde(default)]
    pub task_summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteRouteExecutionEnvelope {
    pub generated_at: String,
    pub route: DelegatedRouteDecision,
    #[serde(default)]
    pub operator_id: Option<String>,
    pub approval_policy: String,
    pub max_runtime_secs: u64,
    pub max_delegations: usize,
    pub max_iterations: usize,
    #[serde(default)]
    pub command_env_allowlist: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct AgentRoutePolicyService;

impl AgentRoutePolicyService {
    pub fn new() -> Self {
        Self
    }

    pub fn resolve_route(
        &self,
        request: &DelegatedRouteRequest,
        signals: &[FabricRouteSignal],
        decided_at: String,
    ) -> DelegatedRouteDecision {
        let filtered = signals
            .iter()
            .filter(|signal| {
                request
                    .preferred_host_id
                    .as_ref()
                    .is_none_or(|host_id| &signal.host_id == host_id)
                    && request
                        .preferred_backend_id
                        .as_ref()
                        .is_none_or(|backend_id| &signal.backend_id == backend_id)
                    && request
                        .preferred_provider_id
                        .as_ref()
                        .is_none_or(|provider_id| {
                            signal.provider_id.as_deref() == Some(provider_id.as_str())
                        })
            })
            .cloned()
            .collect::<Vec<_>>();

        let candidates = if filtered.is_empty() {
            signals.to_vec()
        } else {
            filtered
        };

        let candidate_receipts = candidates
            .iter()
            .map(|candidate| DelegatedRouteCandidateReceipt {
                host_scope: candidate.host_scope.clone(),
                host_id: candidate.host_id.clone(),
                backend_id: candidate.backend_id.clone(),
                provider_id: candidate.provider_id.clone(),
                routeable: candidate.routeable,
                reason: if candidate.routeable {
                    candidate.route_reason.clone()
                } else {
                    candidate
                        .readiness_reason
                        .clone()
                        .unwrap_or_else(|| candidate.route_reason.clone())
                },
            })
            .collect::<Vec<_>>();

        let selected = candidates
            .iter()
            .filter(|candidate| candidate.routeable)
            .min_by_key(|candidate| {
                let scope_rank = if candidate.host_scope == "local" {
                    0
                } else {
                    1
                };
                (
                    scope_rank,
                    candidate.host_id.clone(),
                    candidate.backend_id.clone(),
                )
            });

        let recovery_hints = self.recovery_hints(request, signals, &candidate_receipts);
        let receipt_id = format!("route-{}", decided_at.replace([':', '.'], "-"));

        if let Some(selected) = selected {
            DelegatedRouteDecision {
                receipt_id,
                decided_at,
                status: "selected".to_string(),
                requested_backend_id: request.preferred_backend_id.clone(),
                requested_provider_id: request.preferred_provider_id.clone(),
                requested_host_id: request.preferred_host_id.clone(),
                selected_host_scope: Some(selected.host_scope.clone()),
                selected_host_id: Some(selected.host_id.clone()),
                selected_host_label: Some(selected.host_label.clone()),
                selected_backend_id: Some(selected.backend_id.clone()),
                selected_provider_id: selected.provider_id.clone(),
                reason: selected.route_reason.clone(),
                recovery_hints,
                candidates: candidate_receipts,
                operator_id: request.operator_id.clone(),
                task_summary: request.task_summary.clone(),
            }
        } else {
            DelegatedRouteDecision {
                receipt_id,
                decided_at,
                status: "blocked".to_string(),
                requested_backend_id: request.preferred_backend_id.clone(),
                requested_provider_id: request.preferred_provider_id.clone(),
                requested_host_id: request.preferred_host_id.clone(),
                selected_host_scope: None,
                selected_host_id: None,
                selected_host_label: None,
                selected_backend_id: None,
                selected_provider_id: None,
                reason: "No routeable delegated backend matched the current request.".to_string(),
                recovery_hints,
                candidates: candidate_receipts,
                operator_id: request.operator_id.clone(),
                task_summary: request.task_summary.clone(),
            }
        }
    }

    pub fn build_remote_envelope(
        &self,
        decision: DelegatedRouteDecision,
        approval_policy: &str,
        max_runtime_secs: u64,
        max_delegations: usize,
        max_iterations: usize,
        command_env_allowlist: &[String],
        generated_at: String,
    ) -> Option<RemoteRouteExecutionEnvelope> {
        if decision.selected_host_scope.as_deref() != Some("remote") {
            return None;
        }
        Some(RemoteRouteExecutionEnvelope {
            generated_at,
            operator_id: decision.operator_id.clone(),
            route: decision,
            approval_policy: approval_policy.to_string(),
            max_runtime_secs,
            max_delegations,
            max_iterations,
            command_env_allowlist: command_env_allowlist.to_vec(),
        })
    }

    fn recovery_hints(
        &self,
        request: &DelegatedRouteRequest,
        signals: &[FabricRouteSignal],
        candidates: &[DelegatedRouteCandidateReceipt],
    ) -> Vec<String> {
        let mut hints = Vec::new();
        if signals.iter().all(|signal| signal.host_scope == "local") {
            hints.push(
                "Enroll a trusted remote host with `openrustclaw control fabric-enroll` to expand route options beyond this machine."
                    .to_string(),
            );
        }
        if let Some(host_id) = request.preferred_host_id.as_deref()
            && !signals.iter().any(|signal| signal.host_id == host_id)
        {
            hints.push(format!(
                "Preferred host `{host_id}` is not enrolled. Export inventory from that machine and enroll it into the agent fabric first."
            ));
        }
        if let Some(backend_id) = request.preferred_backend_id.as_deref()
            && !signals.iter().any(|signal| signal.backend_id == backend_id)
        {
            hints.push(format!(
                "Preferred backend `{backend_id}` is not visible in the current fabric inventory."
            ));
        }
        for candidate in candidates
            .iter()
            .filter(|candidate| !candidate.routeable)
            .take(3)
        {
            hints.push(format!(
                "`{}` on `{}` is currently blocked: {}",
                candidate.backend_id, candidate.host_id, candidate.reason
            ));
        }
        hints.sort();
        hints.dedup();
        hints
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_backend_catalog::AgentBackendReadiness;

    fn signal(
        host_scope: &str,
        host_id: &str,
        backend_id: &str,
        routeable: bool,
    ) -> FabricRouteSignal {
        FabricRouteSignal {
            host_scope: host_scope.to_string(),
            host_id: host_id.to_string(),
            host_label: host_id.to_string(),
            backend_id: backend_id.to_string(),
            display_name: backend_id.to_string(),
            provider_id: Some("openai".to_string()),
            transport: if routeable {
                "local_cli_wrapper".to_string()
            } else {
                "unsupported".to_string()
            },
            readiness: if routeable {
                AgentBackendReadiness::Ready
            } else {
                AgentBackendReadiness::Candidate
            },
            readiness_reason: (!routeable).then(|| "login required".to_string()),
            execution_eligible: routeable,
            routeable,
            compatibility: "provider-linked".to_string(),
            route_reason: if routeable {
                "routeable".to_string()
            } else {
                "blocked".to_string()
            },
        }
    }

    #[test]
    fn resolve_route_prefers_local_routeable_candidate() {
        let service = AgentRoutePolicyService::new();
        let decision = service.resolve_route(
            &DelegatedRouteRequest::default(),
            &[
                signal("remote", "remote-a", "codex", true),
                signal("local", "local", "cursor", true),
            ],
            "2026-04-09T00:20:00Z".to_string(),
        );

        assert_eq!(decision.status, "selected");
        assert_eq!(decision.selected_host_scope.as_deref(), Some("local"));
        assert_eq!(decision.selected_backend_id.as_deref(), Some("cursor"));
    }

    #[test]
    fn build_remote_envelope_only_for_remote_selection() {
        let service = AgentRoutePolicyService::new();
        let decision = service.resolve_route(
            &DelegatedRouteRequest {
                preferred_host_id: Some("remote-a".to_string()),
                ..DelegatedRouteRequest::default()
            },
            &[signal("remote", "remote-a", "codex", true)],
            "2026-04-09T00:20:00Z".to_string(),
        );
        let envelope = service.build_remote_envelope(
            decision,
            "side_effects",
            600,
            4,
            8,
            &["PATH".to_string()],
            "2026-04-09T00:20:01Z".to_string(),
        );

        assert!(envelope.is_some());
        assert_eq!(
            envelope.unwrap().route.selected_host_scope.as_deref(),
            Some("remote")
        );
    }
}
