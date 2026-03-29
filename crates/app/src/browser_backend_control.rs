use openrustclaw_core::config::AppConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExternalBackendPolicy {
    pub allowed_backends: Vec<String>,
    pub allow_local_cli_wrappers: bool,
    pub allow_cloud_agent_execution: bool,
    pub audit_log_path: String,
    pub command_env_allowlist: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExternalBackendAuditEntry {
    pub timestamp: String,
    pub backend: String,
    pub transport: String,
    pub action: String,
    pub session_id: Option<String>,
    pub allowed: bool,
    pub success: bool,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackendExecutionPolicyDecision {
    pub allowed: bool,
    pub audit_entry: Option<ExternalBackendAuditEntry>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct BrowserBackendControlService;

impl BrowserBackendControlService {
    pub fn new() -> Self {
        Self
    }

    pub fn normalize_backend_name(&self, raw: &str) -> String {
        raw.trim().to_ascii_lowercase().replace('-', "_")
    }

    pub fn policy_from_config(
        &self,
        config: &AppConfig,
        audit_log_path: String,
    ) -> ExternalBackendPolicy {
        let mut allowed_backends: Vec<String> = config
            .external_backends
            .allowed_backends
            .iter()
            .map(|value| self.normalize_backend_name(value))
            .collect();
        allowed_backends.sort();
        allowed_backends.dedup();
        ExternalBackendPolicy {
            allowed_backends,
            allow_local_cli_wrappers: config.external_backends.allow_local_cli_wrappers,
            allow_cloud_agent_execution: config.external_backends.allow_cloud_agent_execution,
            audit_log_path,
            command_env_allowlist: config.external_backends.command_env_allowlist.clone(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn audit_entry(
        &self,
        backend: &str,
        action: &str,
        session_id: Option<&str>,
        allowed: bool,
        success: bool,
        detail: Option<String>,
        timestamp: String,
    ) -> ExternalBackendAuditEntry {
        ExternalBackendAuditEntry {
            timestamp,
            backend: backend.to_string(),
            transport: if backend == "agent_browser_cli" {
                "local_cli_wrapper".to_string()
            } else {
                "native".to_string()
            },
            action: action.to_string(),
            session_id: session_id.map(ToString::to_string),
            allowed,
            success,
            detail,
        }
    }

    pub fn evaluate_backend_execution(
        &self,
        policy: &ExternalBackendPolicy,
        backend: &str,
        action: &str,
        session_id: Option<&str>,
        timestamp: String,
    ) -> BackendExecutionPolicyDecision {
        if backend != "agent_browser_cli" {
            return BackendExecutionPolicyDecision {
                allowed: true,
                audit_entry: None,
                error_message: None,
            };
        }
        if !policy.allow_local_cli_wrappers {
            let detail = "local CLI wrapper execution is disabled by policy".to_string();
            return BackendExecutionPolicyDecision {
                allowed: false,
                audit_entry: Some(self.audit_entry(
                    backend,
                    action,
                    session_id,
                    false,
                    false,
                    Some(detail.clone()),
                    timestamp,
                )),
                error_message: Some(
                    "agent-browser backend is disabled by external backend policy".to_string(),
                ),
            };
        }
        if !policy.allowed_backends.iter().any(|value| value == backend) {
            let detail = "backend is not in the external backend allowlist".to_string();
            return BackendExecutionPolicyDecision {
                allowed: false,
                audit_entry: Some(self.audit_entry(
                    backend,
                    action,
                    session_id,
                    false,
                    false,
                    Some(detail.clone()),
                    timestamp,
                )),
                error_message: Some(
                    "agent-browser backend is not in the external backend allowlist".to_string(),
                ),
            };
        }
        BackendExecutionPolicyDecision {
            allowed: true,
            audit_entry: None,
            error_message: None,
        }
    }

    pub fn sorted_audit_entries(
        &self,
        mut entries: Vec<ExternalBackendAuditEntry>,
        limit: usize,
    ) -> Vec<ExternalBackendAuditEntry> {
        entries.sort_by(|left, right| right.timestamp.cmp(&left.timestamp));
        entries.truncate(limit.max(1));
        entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_normalizes_and_deduplicates_backends() {
        let mut config = AppConfig::default();
        config.external_backends.allowed_backends = vec![
            "Agent-Browser".to_string(),
            "agent_browser_cli".to_string(),
            "native_cdp".to_string(),
        ];
        let policy = BrowserBackendControlService::new()
            .policy_from_config(&config, ".claw/control/audit.jsonl".to_string());
        assert_eq!(
            policy.allowed_backends,
            vec![
                "agent_browser".to_string(),
                "agent_browser_cli".to_string(),
                "native_cdp".to_string()
            ]
        );
    }

    #[test]
    fn denied_agent_backend_returns_audit_entry_and_message() {
        let service = BrowserBackendControlService::new();
        let decision = service.evaluate_backend_execution(
            &ExternalBackendPolicy {
                allowed_backends: Vec::new(),
                allow_local_cli_wrappers: false,
                allow_cloud_agent_execution: false,
                audit_log_path: ".claw/browser/audit.jsonl".to_string(),
                command_env_allowlist: Vec::new(),
            },
            "agent_browser_cli",
            "inspect",
            Some("session-1"),
            "2026-03-28T00:00:00Z".to_string(),
        );
        assert!(!decision.allowed);
        assert_eq!(
            decision.error_message.as_deref(),
            Some("agent-browser backend is disabled by external backend policy")
        );
        assert_eq!(
            decision.audit_entry.as_ref().unwrap().transport,
            "local_cli_wrapper"
        );
    }

    #[test]
    fn audit_entries_are_sorted_newest_first() {
        let service = BrowserBackendControlService::new();
        let sorted = service.sorted_audit_entries(
            vec![
                service.audit_entry(
                    "agent_browser_cli",
                    "inspect",
                    None,
                    true,
                    true,
                    None,
                    "2026-03-27T00:00:00Z".to_string(),
                ),
                service.audit_entry(
                    "agent_browser_cli",
                    "pdf",
                    None,
                    true,
                    false,
                    Some("boom".to_string()),
                    "2026-03-28T00:00:00Z".to_string(),
                ),
            ],
            10,
        );
        assert_eq!(sorted[0].action, "pdf");
        assert_eq!(sorted[1].action, "inspect");
    }
}
