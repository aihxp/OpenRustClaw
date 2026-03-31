use serde::{Deserialize, Serialize};

const MODE_SOLO: &str = "solo";
const MODE_TEAM: &str = "team";
const MODE_COMPANY: &str = "company";
const MODE_ENTERPRISE: &str = "enterprise";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OnboardingProfile {
    Standard,
    Advanced,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SetupStep {
    Gateway,
    Channel,
    Model,
    ControlPlane,
    Skill,
    Daemon,
}

impl SetupStep {
    pub fn id(self) -> &'static str {
        match self {
            Self::Gateway => "gateway",
            Self::Channel => "channel",
            Self::Model => "model",
            Self::ControlPlane => "control_plane",
            Self::Skill => "skill",
            Self::Daemon => "daemon",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Gateway => "Gateway Setup",
            Self::Channel => "Channel Setup",
            Self::Model => "AI Model Setup",
            Self::ControlPlane => "Claw Runtime Mode",
            Self::Skill => "Skills Setup",
            Self::Daemon => "System Service",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "gateway" => Some(Self::Gateway),
            "channel" => Some(Self::Channel),
            "model" => Some(Self::Model),
            "control_plane" => Some(Self::ControlPlane),
            "skill" => Some(Self::Skill),
            "daemon" => Some(Self::Daemon),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SetupBootstrapOutcome {
    pub category: String,
    pub target: String,
    pub status: String,
    pub detail: String,
    #[serde(default)]
    pub issue_kind: Option<String>,
    #[serde(default)]
    pub verification_stage: Option<String>,
    #[serde(default)]
    pub suggested_action: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SetupLifecycleState {
    pub status: String,
    pub setup_path: Option<String>,
    pub selected_provider: Option<String>,
    pub selected_access_mode: Option<String>,
    pub selected_primary_model: Option<String>,
    pub selected_primary_model_source: Option<String>,
    pub selected_steps: Vec<String>,
    pub completed_steps: Vec<String>,
    pub blockers: Vec<String>,
    pub next_action: Option<String>,
    pub bootstrap_outcomes: Vec<SetupBootstrapOutcome>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SetupDiagnosticStatus {
    Ok,
    Warning,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SetupDiagnosticCheck {
    pub id: String,
    pub label: String,
    pub status: SetupDiagnosticStatus,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SetupRepairPlan {
    pub steps: Vec<SetupStep>,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct SetupLifecycleService;

impl SetupLifecycleService {
    pub fn new() -> Self {
        Self
    }

    pub fn should_offer_assistant_launch(
        &self,
        healthy: bool,
        stdin_is_terminal: bool,
        provider: Option<&str>,
    ) -> bool {
        healthy && stdin_is_terminal && provider.is_some()
    }

    pub fn default_onboarding_profile_for_mode(&self, mode: &str) -> OnboardingProfile {
        match mode {
            MODE_COMPANY | MODE_ENTERPRISE => OnboardingProfile::Advanced,
            _ => OnboardingProfile::Standard,
        }
    }

    pub fn default_runtime_mode_index(&self, mode: Option<&str>) -> usize {
        match mode.unwrap_or(MODE_SOLO) {
            MODE_TEAM => 1,
            MODE_COMPANY | MODE_ENTERPRISE => 3,
            _ => 0,
        }
    }

    pub fn steps_for_profile(&self, profile: OnboardingProfile) -> Vec<SetupStep> {
        match profile {
            OnboardingProfile::Standard => vec![
                SetupStep::Gateway,
                SetupStep::Channel,
                SetupStep::Model,
                SetupStep::ControlPlane,
            ],
            OnboardingProfile::Advanced => vec![
                SetupStep::Gateway,
                SetupStep::Channel,
                SetupStep::Model,
                SetupStep::ControlPlane,
                SetupStep::Skill,
                SetupStep::Daemon,
            ],
            OnboardingProfile::Custom => Vec::new(),
        }
    }

    pub fn selected_remaining_steps(&self, setup: &SetupLifecycleState) -> Vec<SetupStep> {
        let selected = setup
            .selected_steps
            .iter()
            .filter(|id| !setup.completed_steps.contains(*id))
            .filter_map(|id| SetupStep::from_id(id))
            .collect::<Vec<_>>();
        if selected.is_empty() {
            self.steps_for_profile(
                self.profile_from_label(setup.setup_path.as_deref().unwrap_or("Standard")),
            )
        } else {
            selected
        }
    }

    pub fn pending_step_names(&self, setup: &SetupLifecycleState) -> Vec<String> {
        setup
            .selected_steps
            .iter()
            .filter(|id| !setup.completed_steps.contains(*id))
            .filter_map(|id| SetupStep::from_id(id))
            .map(|step| step.name().to_string())
            .collect()
    }

    pub fn non_ready_bootstrap_outcomes(
        &self,
        setup: &SetupLifecycleState,
    ) -> Vec<SetupBootstrapOutcome> {
        setup
            .bootstrap_outcomes
            .iter()
            .filter(|outcome| outcome.status != "ready")
            .cloned()
            .collect()
    }

    pub fn handoff_status(&self, setup: &SetupLifecycleState) -> &'static str {
        if setup.status == "blocked"
            || setup
                .bootstrap_outcomes
                .iter()
                .any(|outcome| outcome.status == "blocked")
        {
            "blocked"
        } else if setup
            .bootstrap_outcomes
            .iter()
            .any(|outcome| outcome.status == "warning")
        {
            "degraded"
        } else if matches!(setup.status.as_str(), "ready" | "completed") {
            "ready"
        } else if setup.status == "in_progress" {
            "in_progress"
        } else {
            "unknown"
        }
    }

    pub fn handoff_detail(&self, setup: &SetupLifecycleState) -> String {
        match self.handoff_status(setup) {
            "blocked" => self
                .non_ready_bootstrap_outcomes(setup)
                .first()
                .map(Self::bootstrap_attention_detail)
                .or_else(|| setup.blockers.first().cloned())
                .or_else(|| setup.next_action.clone())
                .unwrap_or_else(|| "Setup is blocked and needs operator attention.".to_string()),
            "degraded" => self
                .non_ready_bootstrap_outcomes(setup)
                .first()
                .map(Self::bootstrap_attention_detail)
                .or_else(|| setup.next_action.clone())
                .unwrap_or_else(|| {
                    "Setup is usable but still has warning-level bootstrap issues.".to_string()
                }),
            "ready" => "The workspace is ready for first start and assistant handoff.".to_string(),
            "in_progress" => setup
                .next_action
                .clone()
                .unwrap_or_else(|| "Setup is still in progress.".to_string()),
            _ => "Setup state exists but does not yet map to a known handoff status.".to_string(),
        }
    }

    pub fn derive_repair_plan(
        &self,
        setup_state: Option<&SetupLifecycleState>,
        checks: &[SetupDiagnosticCheck],
    ) -> SetupRepairPlan {
        let mut steps = Vec::new();
        let mut reasons = Vec::new();

        if let Some(setup) = setup_state {
            let unfinished_steps = self.selected_remaining_steps(setup);
            for step in &unfinished_steps {
                self.add_repair_step(&mut steps, *step);
            }
            if !unfinished_steps.is_empty() {
                reasons.push("Durable setup state still has unfinished steps.".to_string());
            }

            for outcome in &setup.bootstrap_outcomes {
                if outcome.status == "ready" {
                    continue;
                }
                if let Some(step) = self.repair_step_for_bootstrap_outcome(outcome) {
                    self.add_repair_step(&mut steps, step);
                    reasons.push(format!(
                        "{} bootstrap is {}: {}",
                        outcome.target, outcome.status, outcome.detail
                    ));
                }
            }
        }

        for check in checks {
            if !self.diagnostic_check_requires_repair(check) {
                continue;
            }
            if let Some(step) = self.repair_step_for_diagnostic_check(check) {
                self.add_repair_step(&mut steps, step);
                reasons.push(format!(
                    "{} requires repair: {}",
                    check.label,
                    check
                        .message
                        .clone()
                        .unwrap_or_else(|| "diagnostic reported a blocker".to_string())
                ));
            }
        }

        SetupRepairPlan { steps, reasons }
    }

    fn profile_from_label(&self, label: &str) -> OnboardingProfile {
        match label {
            "Advanced" => OnboardingProfile::Advanced,
            "Custom" => OnboardingProfile::Custom,
            _ => OnboardingProfile::Standard,
        }
    }

    fn diagnostic_check_requires_repair(&self, check: &SetupDiagnosticCheck) -> bool {
        match check.id.as_str() {
            _ if check.status == SetupDiagnosticStatus::Failed => true,
            "api_keys" | "control_registry" | "onboarding_state" => {
                check.status != SetupDiagnosticStatus::Ok
            }
            "channel_readiness" => {
                if check.status == SetupDiagnosticStatus::Ok {
                    false
                } else {
                    let message = check.message.as_deref().unwrap_or_default();
                    !message.contains("No shipped channels are enabled")
                }
            }
            _ => false,
        }
    }

    fn repair_step_for_bootstrap_outcome(
        &self,
        outcome: &SetupBootstrapOutcome,
    ) -> Option<SetupStep> {
        match (outcome.category.as_str(), outcome.target.as_str()) {
            ("runtime", "gateway") => Some(SetupStep::Gateway),
            ("provider", _) => Some(SetupStep::Model),
            ("runtime", "control_plane_lane") => Some(SetupStep::Model),
            ("runtime", "execution_mode") => Some(SetupStep::ControlPlane),
            ("channel", _) => Some(SetupStep::Channel),
            _ => None,
        }
    }

    fn repair_step_for_diagnostic_check(&self, check: &SetupDiagnosticCheck) -> Option<SetupStep> {
        match check.id.as_str() {
            "api_keys" => Some(SetupStep::Model),
            "channel_readiness" => Some(SetupStep::Channel),
            "control_registry" => Some(SetupStep::ControlPlane),
            "onboarding_state" => Some(SetupStep::Model),
            _ => None,
        }
    }

    fn add_repair_step(&self, steps: &mut Vec<SetupStep>, step: SetupStep) {
        if !steps.contains(&step) {
            steps.push(step);
        }
    }

    fn bootstrap_attention_detail(outcome: &SetupBootstrapOutcome) -> String {
        if outcome.verification_stage.as_deref() == Some("provider_connection") {
            let label = outcome
                .issue_kind
                .as_deref()
                .map(bootstrap_issue_label)
                .unwrap_or("verification blocked");
            return format!(
                "Provider verification for `{}` is {}: {}",
                outcome.target, label, outcome.detail
            );
        }

        format!(
            "{} {} needs review: {}",
            outcome.category, outcome.target, outcome.detail
        )
    }
}

fn bootstrap_issue_label(issue_kind: &str) -> &'static str {
    match issue_kind {
        "auth" => "authentication or access blocked",
        "local_runtime_missing" => "local runtime unavailable",
        "model_unavailable" => "model unavailable",
        "billing" => "billing or quota blocked",
        "rate_limited" => "rate-limited",
        "provider_unavailable" => "provider unreachable",
        "not_configured" => "not configured",
        _ => "verification blocked",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enterprise_defaults_to_advanced_profile() {
        let service = SetupLifecycleService::new();
        assert_eq!(
            service.default_onboarding_profile_for_mode(MODE_ENTERPRISE),
            OnboardingProfile::Advanced
        );
        assert_eq!(
            service.default_onboarding_profile_for_mode(MODE_SOLO),
            OnboardingProfile::Standard
        );
    }

    #[test]
    fn profile_steps_match_expected_lengths() {
        let service = SetupLifecycleService::new();
        assert_eq!(
            service.steps_for_profile(OnboardingProfile::Standard).len(),
            4
        );
        assert_eq!(
            service.steps_for_profile(OnboardingProfile::Advanced).len(),
            6
        );
        assert!(
            service
                .steps_for_profile(OnboardingProfile::Custom)
                .is_empty()
        );
    }

    #[test]
    fn handoff_detail_uses_warning_outcome_for_degraded_state() {
        let service = SetupLifecycleService::new();
        let setup = SetupLifecycleState {
            status: "ready".to_string(),
            setup_path: Some("Advanced".to_string()),
            selected_provider: Some("anthropic".to_string()),
            selected_access_mode: Some("api_key".to_string()),
            selected_primary_model: None,
            selected_primary_model_source: None,
            selected_steps: vec!["gateway".to_string()],
            completed_steps: vec!["gateway".to_string()],
            blockers: Vec::new(),
            next_action: Some("Review provider".to_string()),
            bootstrap_outcomes: vec![SetupBootstrapOutcome {
                category: "provider".to_string(),
                target: "anthropic".to_string(),
                status: "warning".to_string(),
                detail: "provider reachable but degraded".to_string(),
                issue_kind: None,
                verification_stage: None,
                suggested_action: None,
                updated_at: "2026-03-28T00:00:00Z".to_string(),
            }],
        };
        assert_eq!(service.handoff_status(&setup), "degraded");
        assert!(
            service
                .handoff_detail(&setup)
                .contains("provider anthropic needs review")
        );
    }

    #[test]
    fn derive_repair_plan_uses_outcomes_and_diagnostics() {
        let service = SetupLifecycleService::new();
        let setup = SetupLifecycleState {
            status: "blocked".to_string(),
            setup_path: Some("Advanced".to_string()),
            selected_provider: Some("anthropic".to_string()),
            selected_access_mode: Some("api_key".to_string()),
            selected_primary_model: None,
            selected_primary_model_source: None,
            selected_steps: vec![
                "gateway".to_string(),
                "model".to_string(),
                "channel".to_string(),
            ],
            completed_steps: vec!["gateway".to_string()],
            blockers: vec!["Provider bootstrap failed".to_string()],
            next_action: Some("Review AI Model Setup and rerun onboarding".to_string()),
            bootstrap_outcomes: vec![
                SetupBootstrapOutcome {
                    category: "provider".to_string(),
                    target: "anthropic".to_string(),
                    status: "blocked".to_string(),
                    detail: "provider not ready".to_string(),
                    issue_kind: None,
                    verification_stage: None,
                    suggested_action: None,
                    updated_at: "2026-03-28T00:00:00Z".to_string(),
                },
                SetupBootstrapOutcome {
                    category: "channel".to_string(),
                    target: "slack".to_string(),
                    status: "warning".to_string(),
                    detail: "channel probe failed".to_string(),
                    issue_kind: None,
                    verification_stage: None,
                    suggested_action: None,
                    updated_at: "2026-03-28T00:00:00Z".to_string(),
                },
            ],
        };
        let checks = vec![
            SetupDiagnosticCheck {
                id: "api_keys".to_string(),
                label: "provider API keys".to_string(),
                status: SetupDiagnosticStatus::Failed,
                message: Some("Anthropic key missing".to_string()),
            },
            SetupDiagnosticCheck {
                id: "channel_readiness".to_string(),
                label: "channel readiness".to_string(),
                status: SetupDiagnosticStatus::Failed,
                message: Some("Slack auth probe failed".to_string()),
            },
        ];

        let plan = service.derive_repair_plan(Some(&setup), &checks);
        assert_eq!(plan.steps, vec![SetupStep::Model, SetupStep::Channel]);
        assert!(
            plan.reasons
                .iter()
                .any(|reason| reason.contains("unfinished steps"))
        );
        assert!(
            plan.reasons
                .iter()
                .any(|reason| reason.contains("provider not ready"))
        );
        assert!(
            plan.reasons
                .iter()
                .any(|reason| reason.contains("Slack auth probe failed"))
        );
    }

    #[test]
    fn launch_offer_requires_health_terminal_and_provider() {
        let service = SetupLifecycleService::new();
        assert!(service.should_offer_assistant_launch(true, true, Some("openrouter")));
        assert!(!service.should_offer_assistant_launch(false, true, Some("openrouter")));
        assert!(!service.should_offer_assistant_launch(true, false, Some("openrouter")));
        assert!(!service.should_offer_assistant_launch(true, true, None));
    }

    #[test]
    fn handoff_detail_surfaces_provider_verification_issue_kind() {
        let service = SetupLifecycleService::new();
        let setup = SetupLifecycleState {
            status: "blocked".to_string(),
            setup_path: Some("Advanced".to_string()),
            selected_provider: Some("ollama".to_string()),
            selected_access_mode: Some("local_runtime".to_string()),
            selected_primary_model: None,
            selected_primary_model_source: None,
            selected_steps: vec!["model".to_string()],
            completed_steps: Vec::new(),
            blockers: vec!["AI Model Setup failed".to_string()],
            next_action: Some("Start Ollama and rerun onboarding.".to_string()),
            bootstrap_outcomes: vec![SetupBootstrapOutcome {
                category: "provider".to_string(),
                target: "ollama".to_string(),
                status: "blocked".to_string(),
                detail: "Local Ollama is not reachable at http://localhost:11434.".to_string(),
                issue_kind: Some("local_runtime_missing".to_string()),
                verification_stage: Some("provider_connection".to_string()),
                suggested_action: Some("Start Ollama and rerun onboarding.".to_string()),
                updated_at: "2026-03-30T00:00:00Z".to_string(),
            }],
        };

        assert_eq!(service.handoff_status(&setup), "blocked");
        assert!(
            service
                .handoff_detail(&setup)
                .contains("local runtime unavailable")
        );
    }
}
