use openrustclaw_core::error::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnterpriseAdminAccessState {
    #[serde(default)]
    pub organization_id: Option<String>,
    pub operator_count: usize,
    pub explicit_identity_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnterpriseAdminPolicyState {
    pub approval_policy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnterpriseAdminAutonomyState {
    pub status: String,
    pub governance_scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnterpriseAdminRunState {
    pub intervention_required: bool,
    pub pause_requested: bool,
    pub kill_requested: bool,
    pub has_last_error: bool,
    pub lifecycle_state: String,
    pub escalation_requested: bool,
    pub rollback_requested: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnterpriseAdminState {
    pub access: EnterpriseAdminAccessState,
    pub policy: EnterpriseAdminPolicyState,
    pub autonomy: EnterpriseAdminAutonomyState,
    #[serde(default)]
    pub active_runs: Vec<EnterpriseAdminRunState>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnterpriseAdminSupervisionSummary {
    pub active_run_count: usize,
    pub attention_required_count: usize,
    pub escalated_count: usize,
    pub rollback_requested_count: usize,
    pub route_hint: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnterpriseAdminPresentation {
    pub status: String,
    pub detail: String,
    pub requires_operator_headers: bool,
    pub supervision: EnterpriseAdminSupervisionSummary,
}

pub struct EnterpriseAdminService {
    state: EnterpriseAdminState,
}

impl EnterpriseAdminService {
    pub fn new(state: EnterpriseAdminState) -> Self {
        Self { state }
    }

    pub fn report(&self) -> Result<EnterpriseAdminPresentation> {
        let active_run_count = self.state.active_runs.len();
        let attention_required_count = self
            .state
            .active_runs
            .iter()
            .filter(|run| {
                run.intervention_required
                    || run.pause_requested
                    || run.kill_requested
                    || run.has_last_error
                    || run.lifecycle_state != "active"
            })
            .count();
        let escalated_count = self
            .state
            .active_runs
            .iter()
            .filter(|run| run.escalation_requested || run.lifecycle_state == "escalated")
            .count();
        let rollback_requested_count = self
            .state
            .active_runs
            .iter()
            .filter(|run| run.rollback_requested || run.lifecycle_state == "rollback_requested")
            .count();

        let supervision = EnterpriseAdminSupervisionSummary {
            active_run_count,
            attention_required_count,
            escalated_count,
            rollback_requested_count,
            route_hint: "/control/ui".to_string(),
            detail: if active_run_count == 0 {
                "No active supervised orchestration runs currently need enterprise operator attention."
                    .to_string()
            } else {
                format!(
                    "{} active supervised run(s), {} needing attention, {} escalated, {} waiting on rollback handling.",
                    active_run_count,
                    attention_required_count,
                    escalated_count,
                    rollback_requested_count
                )
            },
        };

        let requires_operator_headers = self.state.access.explicit_identity_required;
        let detail = if requires_operator_headers {
            format!(
                "Enterprise admin is live for organization `{}` with {} operator(s). Use `/control/ui` with the scoped operator headers to manage policy, identity, governance, audit export, supervised-runtime controls, and the explicit full-autonomy lane from one shipped surface.",
                self.state.access.organization_id.as_deref().unwrap_or("-"),
                self.state.access.operator_count
            )
        } else {
            "Enterprise admin is not bootstrapped yet. Bootstrap enterprise access first, then use the same shipped surface to manage policy, governance, audit export, and supervised-runtime controls under scoped operator identity.".to_string()
        };

        Ok(EnterpriseAdminPresentation {
            status: if requires_operator_headers {
                "ok".to_string()
            } else {
                "bootstrap_required".to_string()
            },
            detail,
            requires_operator_headers,
            supervision,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enterprise_admin_reports_bootstrap_required_without_access_boundary() -> Result<()> {
        let report = EnterpriseAdminService::new(EnterpriseAdminState {
            access: EnterpriseAdminAccessState {
                organization_id: None,
                operator_count: 0,
                explicit_identity_required: false,
            },
            policy: EnterpriseAdminPolicyState {
                approval_policy: "side_effects".to_string(),
            },
            autonomy: EnterpriseAdminAutonomyState {
                status: "bootstrap_required".to_string(),
                governance_scope: "enterprise.full_autonomy.manage".to_string(),
            },
            active_runs: Vec::new(),
        })
        .report()?;

        assert_eq!(report.status, "bootstrap_required");
        assert!(!report.requires_operator_headers);
        assert_eq!(report.supervision.active_run_count, 0);
        assert_eq!(report.supervision.attention_required_count, 0);
        Ok(())
    }

    #[test]
    fn enterprise_admin_reports_attention_and_bootstrapped_status() -> Result<()> {
        let report = EnterpriseAdminService::new(EnterpriseAdminState {
            access: EnterpriseAdminAccessState {
                organization_id: Some("acme".to_string()),
                operator_count: 2,
                explicit_identity_required: true,
            },
            policy: EnterpriseAdminPolicyState {
                approval_policy: "side_effects".to_string(),
            },
            autonomy: EnterpriseAdminAutonomyState {
                status: "ok".to_string(),
                governance_scope: "enterprise.full_autonomy.manage".to_string(),
            },
            active_runs: vec![
                EnterpriseAdminRunState {
                    intervention_required: false,
                    pause_requested: false,
                    kill_requested: false,
                    has_last_error: false,
                    lifecycle_state: "active".to_string(),
                    escalation_requested: false,
                    rollback_requested: false,
                },
                EnterpriseAdminRunState {
                    intervention_required: true,
                    pause_requested: false,
                    kill_requested: false,
                    has_last_error: false,
                    lifecycle_state: "escalated".to_string(),
                    escalation_requested: true,
                    rollback_requested: true,
                },
            ],
        })
        .report()?;

        assert_eq!(report.status, "ok");
        assert!(report.requires_operator_headers);
        assert!(report.detail.contains("organization `acme`"));
        assert_eq!(report.supervision.active_run_count, 2);
        assert_eq!(report.supervision.attention_required_count, 1);
        assert_eq!(report.supervision.escalated_count, 1);
        assert_eq!(report.supervision.rollback_requested_count, 1);
        Ok(())
    }
}
