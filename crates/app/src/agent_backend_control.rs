use crate::agent_backend_catalog::{
    AgentBackendCapability, AgentBackendCatalogEntry, AgentBackendReadiness,
};
use crate::browser_backend_control::ExternalBackendAuditEntry;
use openrustclaw_core::config::AppConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DelegatedModelCatalogMode {
    Supported,
    VendorManaged,
    Unknown,
    Unsupported,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DelegatedExecutionKind {
    LocalCli,
    Unsupported,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentBackendPolicy {
    pub allowed_backends: Vec<String>,
    pub allow_local_cli_wrappers: bool,
    pub allow_cloud_agent_execution: bool,
    pub audit_log_path: String,
    pub command_env_allowlist: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DelegatedAgentBackendContract {
    pub backend_id: String,
    pub display_name: String,
    pub provider_id: Option<String>,
    pub transport: String,
    pub readiness: AgentBackendReadiness,
    pub readiness_reason: Option<String>,
    pub model_catalog_mode: DelegatedModelCatalogMode,
    pub delegated_execution_kind: DelegatedExecutionKind,
    pub execution_eligible: bool,
    pub policy_classification: String,
    pub auth_method: Option<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentBackendExecutionPolicyDecision {
    pub allowed: bool,
    pub audit_entry: Option<ExternalBackendAuditEntry>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct AgentBackendControlService;

impl AgentBackendControlService {
    pub fn new() -> Self {
        Self
    }

    pub fn normalize_backend_name(&self, raw: &str) -> String {
        raw.trim().to_ascii_lowercase().replace('-', "_")
    }

    pub fn backend_id_for_entry(&self, entry: &AgentBackendCatalogEntry) -> String {
        self.normalize_backend_name(entry.host.as_id())
    }

    pub fn provider_id_for_entry(&self, entry: &AgentBackendCatalogEntry) -> Option<String> {
        match entry.host {
            crate::tool_host_service::AiHost::ClaudeCode => Some("anthropic".to_string()),
            crate::tool_host_service::AiHost::Codex => Some("openai".to_string()),
            crate::tool_host_service::AiHost::GeminiCli => Some("gemini".to_string()),
            _ => None,
        }
    }

    pub fn policy_from_config(
        &self,
        config: &AppConfig,
        audit_log_path: String,
    ) -> AgentBackendPolicy {
        let mut allowed_backends = config
            .external_backends
            .allowed_backends
            .iter()
            .map(|value| self.normalize_backend_name(value))
            .collect::<Vec<_>>();
        allowed_backends.sort();
        allowed_backends.dedup();
        AgentBackendPolicy {
            allowed_backends,
            allow_local_cli_wrappers: config.external_backends.allow_local_cli_wrappers,
            allow_cloud_agent_execution: config.external_backends.allow_cloud_agent_execution,
            audit_log_path,
            command_env_allowlist: config.external_backends.command_env_allowlist.clone(),
        }
    }

    pub fn contract_for_entry(
        &self,
        entry: &AgentBackendCatalogEntry,
    ) -> DelegatedAgentBackendContract {
        let model_catalog_mode = match entry.model_discovery {
            AgentBackendCapability::Supported => DelegatedModelCatalogMode::Supported,
            AgentBackendCapability::Unknown => DelegatedModelCatalogMode::Unknown,
            AgentBackendCapability::Candidate | AgentBackendCapability::Unsupported
                if entry.policy_classification == "delegated_cli_candidate" =>
            {
                DelegatedModelCatalogMode::VendorManaged
            }
            _ => DelegatedModelCatalogMode::Unsupported,
        };
        let delegated_execution_kind = if entry.policy_classification == "delegated_cli_candidate"
            && entry.delegated_execution != AgentBackendCapability::Unsupported
        {
            DelegatedExecutionKind::LocalCli
        } else {
            DelegatedExecutionKind::Unsupported
        };
        let execution_eligible = delegated_execution_kind == DelegatedExecutionKind::LocalCli
            && entry.readiness == AgentBackendReadiness::Ready;

        DelegatedAgentBackendContract {
            backend_id: self.backend_id_for_entry(entry),
            display_name: entry.display_name().to_string(),
            provider_id: self.provider_id_for_entry(entry),
            transport: if delegated_execution_kind == DelegatedExecutionKind::LocalCli {
                "local_cli_wrapper".to_string()
            } else {
                "unsupported".to_string()
            },
            readiness: entry.readiness.clone(),
            readiness_reason: entry.readiness_reason.clone(),
            model_catalog_mode,
            delegated_execution_kind,
            execution_eligible,
            policy_classification: entry.policy_classification.clone(),
            auth_method: entry.auth_method.clone(),
            notes: entry.notes.clone(),
        }
    }

    pub fn contracts_from_catalog(
        &self,
        entries: &[AgentBackendCatalogEntry],
    ) -> Vec<DelegatedAgentBackendContract> {
        entries
            .iter()
            .map(|entry| self.contract_for_entry(entry))
            .collect()
    }

    pub fn audit_entry(
        &self,
        contract: &DelegatedAgentBackendContract,
        action: &str,
        session_id: Option<&str>,
        allowed: bool,
        detail: Option<String>,
        timestamp: String,
    ) -> ExternalBackendAuditEntry {
        ExternalBackendAuditEntry {
            timestamp,
            backend: contract.backend_id.clone(),
            transport: contract.transport.clone(),
            action: action.to_string(),
            session_id: session_id.map(ToString::to_string),
            allowed,
            success: allowed,
            detail,
        }
    }

    pub fn evaluate_execution(
        &self,
        policy: &AgentBackendPolicy,
        contract: &DelegatedAgentBackendContract,
        action: &str,
        session_id: Option<&str>,
        timestamp: String,
    ) -> AgentBackendExecutionPolicyDecision {
        if contract.delegated_execution_kind != DelegatedExecutionKind::LocalCli {
            let detail =
                "backend does not expose a supported delegated local CLI execution contract"
                    .to_string();
            return AgentBackendExecutionPolicyDecision {
                allowed: false,
                audit_entry: Some(self.audit_entry(
                    contract,
                    action,
                    session_id,
                    false,
                    Some(detail.clone()),
                    timestamp,
                )),
                error_message: Some(detail),
            };
        }
        if !policy.allow_local_cli_wrappers {
            let detail = "local CLI wrapper execution is disabled by policy".to_string();
            return AgentBackendExecutionPolicyDecision {
                allowed: false,
                audit_entry: Some(self.audit_entry(
                    contract,
                    action,
                    session_id,
                    false,
                    Some(detail.clone()),
                    timestamp,
                )),
                error_message: Some(detail),
            };
        }
        if !policy
            .allowed_backends
            .iter()
            .any(|entry| entry == &contract.backend_id)
        {
            let detail = "backend is not in the external backend allowlist".to_string();
            return AgentBackendExecutionPolicyDecision {
                allowed: false,
                audit_entry: Some(self.audit_entry(
                    contract,
                    action,
                    session_id,
                    false,
                    Some(detail.clone()),
                    timestamp,
                )),
                error_message: Some(detail),
            };
        }
        if !contract.execution_eligible {
            let detail = contract.readiness_reason.clone().unwrap_or_else(|| {
                "backend is detected, but readiness is not yet sufficient for delegated execution"
                    .to_string()
            });
            return AgentBackendExecutionPolicyDecision {
                allowed: false,
                audit_entry: Some(self.audit_entry(
                    contract,
                    action,
                    session_id,
                    false,
                    Some(detail.clone()),
                    timestamp,
                )),
                error_message: Some(detail),
            };
        }

        AgentBackendExecutionPolicyDecision {
            allowed: true,
            audit_entry: None,
            error_message: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_backend_catalog::{AgentBackendAuthStatus, AgentBackendCapability};
    use crate::tool_host_service::AiHost;

    fn sample_entry(
        host: AiHost,
        readiness: AgentBackendReadiness,
        readiness_reason: Option<&str>,
        policy_classification: &str,
    ) -> AgentBackendCatalogEntry {
        AgentBackendCatalogEntry {
            host,
            binary_name: "tool".to_string(),
            detected: true,
            executable_path: Some("/usr/local/bin/tool".to_string()),
            version_text: Some("1.0.0".to_string()),
            summary: Some("summary".to_string()),
            auth_status: AgentBackendAuthStatus::LoggedIn,
            auth_method: Some("first_party".to_string()),
            model_discovery: AgentBackendCapability::Unsupported,
            delegated_execution: AgentBackendCapability::Candidate,
            policy_classification: policy_classification.to_string(),
            readiness,
            readiness_reason: readiness_reason.map(ToString::to_string),
            detected_subcommands: vec!["run".to_string()],
            detected_flags: vec!["--model".to_string()],
            discovered_models: Vec::new(),
            inspected_at: "2026-04-08T00:00:00Z".to_string(),
            notes: vec!["note".to_string()],
        }
    }

    #[test]
    fn candidate_backends_are_labeled_vendor_managed_when_models_are_not_exposed() {
        let service = AgentBackendControlService::new();
        let contract = service.contract_for_entry(&sample_entry(
            AiHost::ClaudeCode,
            AgentBackendReadiness::Ready,
            None,
            "delegated_cli_candidate",
        ));

        assert_eq!(
            contract.model_catalog_mode,
            DelegatedModelCatalogMode::VendorManaged
        );
        assert_eq!(
            contract.delegated_execution_kind,
            DelegatedExecutionKind::LocalCli
        );
        assert!(contract.execution_eligible);
        assert_eq!(contract.provider_id.as_deref(), Some("anthropic"));
    }

    #[test]
    fn detection_only_backends_remain_ineligible() {
        let service = AgentBackendControlService::new();
        let contract = service.contract_for_entry(&sample_entry(
            AiHost::Cursor,
            AgentBackendReadiness::DetectionOnly,
            Some("documented backend surface not confirmed"),
            "integration_only",
        ));

        assert_eq!(
            contract.delegated_execution_kind,
            DelegatedExecutionKind::Unsupported
        );
        assert!(!contract.execution_eligible);
        assert_eq!(
            contract.model_catalog_mode,
            DelegatedModelCatalogMode::Unsupported
        );
    }

    #[test]
    fn policy_denies_unlisted_or_not_ready_backends() {
        let service = AgentBackendControlService::new();
        let contract = service.contract_for_entry(&sample_entry(
            AiHost::Codex,
            AgentBackendReadiness::Candidate,
            Some("Binary is installed, but the current login status is not ready for delegated execution."),
            "delegated_cli_candidate",
        ));
        let policy = AgentBackendPolicy {
            allowed_backends: vec!["codex".to_string()],
            allow_local_cli_wrappers: true,
            allow_cloud_agent_execution: false,
            audit_log_path: ".claw/control/external-backends-audit.jsonl".to_string(),
            command_env_allowlist: vec!["PATH".to_string()],
        };

        let decision = service.evaluate_execution(
            &policy,
            &contract,
            "delegate",
            Some("session-1"),
            "2026-04-08T00:00:00Z".to_string(),
        );

        assert!(!decision.allowed);
        assert!(decision.audit_entry.is_some());
        assert!(
            decision
                .error_message
                .as_deref()
                .unwrap_or_default()
                .contains("not ready")
        );
    }

    #[test]
    fn policy_allows_ready_allowlisted_local_cli_backends() {
        let service = AgentBackendControlService::new();
        let contract = service.contract_for_entry(&sample_entry(
            AiHost::GeminiCli,
            AgentBackendReadiness::Ready,
            None,
            "delegated_cli_candidate",
        ));
        let policy = AgentBackendPolicy {
            allowed_backends: vec!["gemini_cli".to_string()],
            allow_local_cli_wrappers: true,
            allow_cloud_agent_execution: false,
            audit_log_path: ".claw/control/external-backends-audit.jsonl".to_string(),
            command_env_allowlist: vec!["PATH".to_string()],
        };

        let decision = service.evaluate_execution(
            &policy,
            &contract,
            "delegate",
            Some("session-2"),
            "2026-04-08T00:00:00Z".to_string(),
        );

        assert!(decision.allowed);
        assert!(decision.audit_entry.is_none());
        assert!(decision.error_message.is_none());
    }
}
