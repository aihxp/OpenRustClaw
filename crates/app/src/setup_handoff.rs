use openrustclaw_core::error::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SetupStatus {
    NotStarted,
    Pending,
    Degraded,
    Ready,
}

impl SetupStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SetupStatus::NotStarted => "not_started",
            SetupStatus::Pending => "pending",
            SetupStatus::Degraded => "degraded",
            SetupStatus::Ready => "ready",
        }
    }

    pub fn ready_for_first_start(&self) -> bool {
        matches!(self, SetupStatus::Ready)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteConnectivityProfile {
    pub mode: String,
    pub primary_path: String,
    #[serde(default)]
    pub fallback_paths: Vec<String>,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SetupBootstrapOutcome {
    pub category: String,
    pub target: String,
    pub status: String,
    pub detail: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SetupHandoffState {
    pub manifest_path: String,
    pub status: SetupStatus,
    pub detail: String,
    pub explicit_setup_state: bool,
    pub deployment_mode: Option<String>,
    pub deployment_path: Option<String>,
    pub remote_connectivity_profile: Option<RemoteConnectivityProfile>,
    pub setup_path: Option<String>,
    pub workspace_action: Option<String>,
    pub current_step: Option<String>,
    pub next_action: Option<String>,
    pub selected_step_count: usize,
    pub completed_step_count: usize,
    #[serde(default)]
    pub pending_steps: Vec<String>,
    #[serde(default)]
    pub blockers: Vec<String>,
    #[serde(default)]
    pub bootstrap_outcomes: Vec<SetupBootstrapOutcome>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SetupHandoffReport {
    pub status: String,
    pub detail: String,
    pub manifest_path: String,
    pub explicit_setup_state: bool,
    pub deployment_mode: Option<String>,
    pub deployment_path: Option<String>,
    pub remote_connectivity_profile: Option<RemoteConnectivityProfile>,
    pub setup_path: Option<String>,
    pub workspace_action: Option<String>,
    pub current_step: Option<String>,
    pub next_action: Option<String>,
    pub ready_for_first_start: bool,
    pub selected_step_count: usize,
    pub completed_step_count: usize,
    pub pending_steps: Vec<String>,
    pub blockers: Vec<String>,
    pub bootstrap_outcomes: Vec<SetupBootstrapOutcome>,
}

impl From<SetupHandoffState> for SetupHandoffReport {
    fn from(state: SetupHandoffState) -> Self {
        Self {
            status: state.status.as_str().to_string(),
            detail: state.detail,
            manifest_path: state.manifest_path,
            explicit_setup_state: state.explicit_setup_state,
            deployment_mode: state.deployment_mode,
            deployment_path: state.deployment_path,
            remote_connectivity_profile: state.remote_connectivity_profile,
            setup_path: state.setup_path,
            workspace_action: state.workspace_action,
            current_step: state.current_step,
            next_action: state.next_action,
            ready_for_first_start: state.status.ready_for_first_start(),
            selected_step_count: state.selected_step_count,
            completed_step_count: state.completed_step_count,
            pending_steps: state.pending_steps,
            blockers: state.blockers,
            bootstrap_outcomes: state.bootstrap_outcomes,
        }
    }
}

pub trait SetupHandoffStateSource {
    fn load_setup_handoff_state(&self) -> Result<Option<SetupHandoffState>>;
    fn manifest_path(&self) -> String;
}

pub struct SetupHandoffService<S> {
    source: S,
}

impl<S> SetupHandoffService<S> {
    pub fn new(source: S) -> Self {
        Self { source }
    }
}

impl<S> SetupHandoffService<S>
where
    S: SetupHandoffStateSource,
{
    pub fn report(&self) -> Result<SetupHandoffReport> {
        if let Some(state) = self.source.load_setup_handoff_state()? {
            Ok(state.into())
        } else {
            Ok(SetupHandoffReport {
                status: SetupStatus::NotStarted.as_str().to_string(),
                detail: "No durable setup state is recorded yet. Run `openrustclaw onboard` to create the setup contract before first start.".to_string(),
                manifest_path: self.source.manifest_path(),
                explicit_setup_state: false,
                deployment_mode: None,
                deployment_path: None,
                remote_connectivity_profile: None,
                setup_path: None,
                workspace_action: None,
                current_step: None,
                next_action: Some("Run `openrustclaw onboard`.".to_string()),
                ready_for_first_start: false,
                selected_step_count: 0,
                completed_step_count: 0,
                pending_steps: Vec::new(),
                blockers: Vec::new(),
                bootstrap_outcomes: Vec::new(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubSource {
        manifest_path: String,
        state: Option<SetupHandoffState>,
    }

    impl SetupHandoffStateSource for StubSource {
        fn load_setup_handoff_state(&self) -> Result<Option<SetupHandoffState>> {
            Ok(self.state.clone())
        }

        fn manifest_path(&self) -> String {
            self.manifest_path.clone()
        }
    }

    #[test]
    fn report_defaults_to_not_started_when_state_is_missing() -> Result<()> {
        let service = SetupHandoffService::new(StubSource {
            manifest_path: ".claw/control/setup-state.json".to_string(),
            state: None,
        });

        let report = service.report()?;
        assert_eq!(report.status, "not_started");
        assert!(!report.explicit_setup_state);
        assert_eq!(
            report.next_action.as_deref(),
            Some("Run `openrustclaw onboard`.")
        );
        assert!(!report.ready_for_first_start);
        Ok(())
    }

    #[test]
    fn report_preserves_state_and_marks_ready() -> Result<()> {
        let service = SetupHandoffService::new(StubSource {
            manifest_path: ".claw/control/setup-state.json".to_string(),
            state: Some(SetupHandoffState {
                manifest_path: ".claw/control/setup-state.json".to_string(),
                status: SetupStatus::Ready,
                detail: "Everything is configured and ready for first start.".to_string(),
                explicit_setup_state: true,
                deployment_mode: Some("solo".to_string()),
                deployment_path: Some("remote_node".to_string()),
                remote_connectivity_profile: Some(RemoteConnectivityProfile {
                    mode: "advanced".to_string(),
                    primary_path: "node_first".to_string(),
                    fallback_paths: vec![
                        "ssh_tunnel".to_string(),
                        "reverse_proxy".to_string(),
                    ],
                    detail: "Prefer the node path and fall back to SSH tunnel only if needed."
                        .to_string(),
                }),
                setup_path: Some("advanced".to_string()),
                workspace_action: Some("resume".to_string()),
                current_step: Some("doctor".to_string()),
                next_action: Some("Run `openrustclaw start`.".to_string()),
                selected_step_count: 5,
                completed_step_count: 5,
                pending_steps: Vec::new(),
                blockers: Vec::new(),
                bootstrap_outcomes: vec![SetupBootstrapOutcome {
                    category: "gateway".to_string(),
                    target: "control".to_string(),
                    status: "ready".to_string(),
                    detail: "Gateway bootstrap completed.".to_string(),
                    updated_at: "2026-03-28T00:00:00Z".to_string(),
                }],
            }),
        });

        let report = service.report()?;
        assert_eq!(report.status, "ready");
        assert!(report.ready_for_first_start);
        assert_eq!(report.deployment_path.as_deref(), Some("remote_node"));
        assert_eq!(
            report
                .remote_connectivity_profile
                .as_ref()
                .map(|profile| profile.primary_path.as_str()),
            Some("node_first")
        );
        assert_eq!(report.bootstrap_outcomes.len(), 1);
        Ok(())
    }
}
