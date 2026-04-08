use crate::agent_backend_catalog::{AgentBackendCatalogEntry, AgentBackendReadiness};
use crate::agent_backend_control::{
    DelegatedAgentBackendContract, DelegatedExecutionKind, DelegatedModelCatalogMode,
};
use serde::{Deserialize, Serialize};

fn default_version() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentFabricAdvertisement {
    pub host_id: String,
    pub host_label: String,
    pub exported_at: String,
    #[serde(default)]
    pub delegated_backends: Vec<RemoteDelegatedBackendSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteDelegatedBackendSnapshot {
    pub backend_id: String,
    pub display_name: String,
    #[serde(default)]
    pub provider_id: Option<String>,
    pub transport: String,
    pub readiness: AgentBackendReadiness,
    #[serde(default)]
    pub readiness_reason: Option<String>,
    pub model_catalog_mode: DelegatedModelCatalogMode,
    pub delegated_execution_kind: DelegatedExecutionKind,
    pub execution_eligible: bool,
    pub policy_classification: String,
    #[serde(default)]
    pub auth_method: Option<String>,
    #[serde(default)]
    pub notes: Vec<String>,
    #[serde(default)]
    pub discovered_models: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrustedRemoteHostRecord {
    pub host_id: String,
    pub host_label: String,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub enrolled_at: String,
    pub updated_at: String,
    pub advertisement: AgentFabricAdvertisement,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct AgentFabricRegistry {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub hosts: Vec<TrustedRemoteHostRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnrollRemoteHostRequest {
    pub host_id: String,
    pub host_label: String,
    pub base_url: Option<String>,
    pub notes: Option<String>,
    pub enrolled_at: String,
    pub advertisement: AgentFabricAdvertisement,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FabricRouteSignal {
    pub host_scope: String,
    pub host_id: String,
    pub host_label: String,
    pub backend_id: String,
    pub display_name: String,
    #[serde(default)]
    pub provider_id: Option<String>,
    pub transport: String,
    pub readiness: AgentBackendReadiness,
    #[serde(default)]
    pub readiness_reason: Option<String>,
    pub execution_eligible: bool,
    pub routeable: bool,
    pub compatibility: String,
    pub route_reason: String,
}

#[derive(Debug, Clone, Default)]
pub struct AgentFabricRegistryService;

impl AgentFabricRegistryService {
    pub fn new() -> Self {
        Self
    }

    pub fn export_local_advertisement(
        &self,
        host_id: &str,
        host_label: &str,
        exported_at: String,
        entries: &[AgentBackendCatalogEntry],
        contracts: &[DelegatedAgentBackendContract],
    ) -> AgentFabricAdvertisement {
        let delegated_backends = contracts
            .iter()
            .map(|contract| {
                let discovered_models = entries
                    .iter()
                    .find(|entry| normalize_host_id(entry.host.as_id()) == contract.backend_id)
                    .map(|entry| entry.discovered_models.clone())
                    .unwrap_or_default();
                RemoteDelegatedBackendSnapshot {
                    backend_id: contract.backend_id.clone(),
                    display_name: contract.display_name.clone(),
                    provider_id: contract.provider_id.clone(),
                    transport: contract.transport.clone(),
                    readiness: contract.readiness.clone(),
                    readiness_reason: contract.readiness_reason.clone(),
                    model_catalog_mode: contract.model_catalog_mode.clone(),
                    delegated_execution_kind: contract.delegated_execution_kind.clone(),
                    execution_eligible: contract.execution_eligible,
                    policy_classification: contract.policy_classification.clone(),
                    auth_method: contract.auth_method.clone(),
                    notes: contract.notes.clone(),
                    discovered_models,
                }
            })
            .collect();

        AgentFabricAdvertisement {
            host_id: host_id.to_string(),
            host_label: host_label.to_string(),
            exported_at,
            delegated_backends,
        }
    }

    pub fn enroll_remote_host(
        &self,
        registry: &mut AgentFabricRegistry,
        request: EnrollRemoteHostRequest,
    ) {
        let record = TrustedRemoteHostRecord {
            host_id: request.host_id,
            host_label: request.host_label,
            base_url: request.base_url,
            notes: request.notes,
            enabled: true,
            enrolled_at: request.enrolled_at.clone(),
            updated_at: request.enrolled_at,
            advertisement: request.advertisement,
        };
        if let Some(existing) = registry
            .hosts
            .iter_mut()
            .find(|host| host.host_id == record.host_id)
        {
            *existing = record;
        } else {
            registry.hosts.push(record);
            registry
                .hosts
                .sort_by(|left, right| left.host_id.cmp(&right.host_id));
        }
    }

    pub fn route_signals(
        &self,
        local_advertisement: &AgentFabricAdvertisement,
        registry: &AgentFabricRegistry,
    ) -> Vec<FabricRouteSignal> {
        let mut signals = local_advertisement
            .delegated_backends
            .iter()
            .map(|backend| {
                self.signal_from_snapshot(
                    "local",
                    &local_advertisement.host_id,
                    &local_advertisement.host_label,
                    true,
                    backend,
                )
            })
            .collect::<Vec<_>>();

        for host in registry.hosts.iter().filter(|host| host.enabled) {
            for backend in &host.advertisement.delegated_backends {
                signals.push(self.signal_from_snapshot(
                    "remote",
                    &host.host_id,
                    &host.host_label,
                    false,
                    backend,
                ));
            }
        }

        signals.sort_by(|left, right| {
            left.host_scope
                .cmp(&right.host_scope)
                .then(left.host_id.cmp(&right.host_id))
                .then(left.backend_id.cmp(&right.backend_id))
        });
        signals
    }

    fn signal_from_snapshot(
        &self,
        host_scope: &str,
        host_id: &str,
        host_label: &str,
        local_host: bool,
        backend: &RemoteDelegatedBackendSnapshot,
    ) -> FabricRouteSignal {
        let compatibility = match (&backend.provider_id, &backend.model_catalog_mode) {
            (Some(provider_id), DelegatedModelCatalogMode::Supported) => {
                format!("provider-linked ({provider_id}) with documented model discovery")
            }
            (Some(provider_id), DelegatedModelCatalogMode::VendorManaged) => {
                format!("provider-linked ({provider_id}) with vendor-managed models")
            }
            (None, DelegatedModelCatalogMode::Supported) => {
                "multi-model delegated backend with documented model discovery".to_string()
            }
            (None, DelegatedModelCatalogMode::VendorManaged) => {
                "multi-model delegated backend with vendor-managed model choice".to_string()
            }
            (_, DelegatedModelCatalogMode::Unknown) => {
                "compatibility is visible, but model discovery is still untrusted".to_string()
            }
            (_, DelegatedModelCatalogMode::Unsupported) => {
                "visible for inventory only; this backend is not routeable yet".to_string()
            }
        };
        let routeable = backend.execution_eligible;
        let route_reason = if routeable {
            if local_host {
                "routeable on this machine under local delegated backend policy".to_string()
            } else {
                "routeable on an explicitly enrolled remote host".to_string()
            }
        } else {
            backend.readiness_reason.clone().unwrap_or_else(|| {
                "backend is enrolled, but readiness is not sufficient for route selection yet"
                    .to_string()
            })
        };

        FabricRouteSignal {
            host_scope: host_scope.to_string(),
            host_id: host_id.to_string(),
            host_label: host_label.to_string(),
            backend_id: backend.backend_id.clone(),
            display_name: backend.display_name.clone(),
            provider_id: backend.provider_id.clone(),
            transport: backend.transport.clone(),
            readiness: backend.readiness.clone(),
            readiness_reason: backend.readiness_reason.clone(),
            execution_eligible: backend.execution_eligible,
            routeable,
            compatibility,
            route_reason,
        }
    }
}

fn normalize_host_id(raw: &str) -> String {
    raw.trim().to_ascii_lowercase().replace('-', "_")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_backend(backend_id: &str, routeable: bool) -> RemoteDelegatedBackendSnapshot {
        RemoteDelegatedBackendSnapshot {
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
            model_catalog_mode: DelegatedModelCatalogMode::VendorManaged,
            delegated_execution_kind: if routeable {
                DelegatedExecutionKind::LocalCli
            } else {
                DelegatedExecutionKind::Unsupported
            },
            execution_eligible: routeable,
            policy_classification: "delegated_cli_candidate".to_string(),
            auth_method: Some("chatgpt".to_string()),
            notes: vec!["note".to_string()],
            discovered_models: vec!["auto".to_string()],
        }
    }

    #[test]
    fn enroll_remote_host_replaces_existing_host_record() {
        let service = AgentFabricRegistryService::new();
        let mut registry = AgentFabricRegistry::default();
        let request = EnrollRemoteHostRequest {
            host_id: "remote-a".to_string(),
            host_label: "Remote A".to_string(),
            base_url: Some("https://remote-a.example".to_string()),
            notes: Some("trusted".to_string()),
            enrolled_at: "2026-04-08T00:00:00Z".to_string(),
            advertisement: AgentFabricAdvertisement {
                host_id: "remote-a".to_string(),
                host_label: "Remote A".to_string(),
                exported_at: "2026-04-08T00:00:00Z".to_string(),
                delegated_backends: vec![sample_backend("codex", true)],
            },
        };
        service.enroll_remote_host(&mut registry, request.clone());
        service.enroll_remote_host(
            &mut registry,
            EnrollRemoteHostRequest {
                host_label: "Remote A Updated".to_string(),
                ..request
            },
        );

        assert_eq!(registry.hosts.len(), 1);
        assert_eq!(registry.hosts[0].host_label, "Remote A Updated");
    }

    #[test]
    fn route_signals_include_local_and_remote_inventory() {
        let service = AgentFabricRegistryService::new();
        let local = AgentFabricAdvertisement {
            host_id: "local".to_string(),
            host_label: "Local".to_string(),
            exported_at: "2026-04-08T00:00:00Z".to_string(),
            delegated_backends: vec![sample_backend("cursor", true)],
        };
        let registry = AgentFabricRegistry {
            version: 1,
            hosts: vec![TrustedRemoteHostRecord {
                host_id: "remote-a".to_string(),
                host_label: "Remote A".to_string(),
                base_url: Some("https://remote-a.example".to_string()),
                notes: None,
                enabled: true,
                enrolled_at: "2026-04-08T00:00:00Z".to_string(),
                updated_at: "2026-04-08T00:00:00Z".to_string(),
                advertisement: AgentFabricAdvertisement {
                    host_id: "remote-a".to_string(),
                    host_label: "Remote A".to_string(),
                    exported_at: "2026-04-08T00:00:00Z".to_string(),
                    delegated_backends: vec![sample_backend("codex", true)],
                },
            }],
        };

        let signals = service.route_signals(&local, &registry);
        assert_eq!(signals.len(), 2);
        assert!(signals.iter().any(|signal| signal.host_scope == "local"));
        assert!(signals.iter().any(|signal| signal.host_scope == "remote"));
    }
}
