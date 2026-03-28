use openrustclaw_core::error::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct OrchestrationRequestOverrides {
    pub model_profile_id: Option<String>,
    pub worker_model_profile_id: Option<String>,
    pub autonomy_level: Option<String>,
    pub max_delegations: Option<usize>,
    pub max_iterations: Option<usize>,
    pub max_runtime_secs: Option<u64>,
    pub approval_policy: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct OrchestrationRequest {
    pub prompt: String,
    pub task_id: Option<String>,
    pub category: Option<String>,
    pub claw_id: Option<String>,
    pub mode: String,
    pub overrides: OrchestrationRequestOverrides,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrchestrationAutonomyPolicy {
    pub autonomy_level: String,
    pub yolo_mode: bool,
    pub steering_enabled: bool,
    pub decision_learning_enabled: bool,
    pub critic_enabled: bool,
    pub max_delegations: usize,
    pub max_iterations: usize,
    pub max_runtime_secs: u64,
    pub max_lesson_hints: usize,
    pub approval_policy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrchestrationRuntimeSpec {
    pub mode: String,
    pub default_claw_id: Option<String>,
    pub orchestrator_claw_id: Option<String>,
    pub task_assignments: Vec<(String, String)>,
    pub category_assignments: Vec<(String, String)>,
    pub allow_shared_context: bool,
    pub isolation_mode: String,
    pub autonomy: OrchestrationAutonomyPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrchestrationClawRouteSpec {
    pub id: String,
    pub role: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrchestrationRouteSelection {
    pub execution_mode: String,
    pub route_source: String,
    pub selected_claw_id: String,
    pub selected_claw_role: String,
    pub available_workers: Vec<String>,
    pub autonomy: OrchestrationAutonomyPolicy,
    pub allow_shared_context: bool,
    pub isolation_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct OrchestrationLifecycleSummary {
    pub state: String,
    pub intervention_required: bool,
    pub escalation_requested: bool,
    pub rollback_requested: bool,
    pub rollback_reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActiveRunInterventionTransition {
    pub stage: String,
    pub status: String,
    pub note: String,
    pub resulting_state: String,
    pub pause_requested: bool,
    pub kill_requested: bool,
    pub lifecycle: OrchestrationLifecycleSummary,
}

#[derive(Debug, Clone, Default)]
pub struct OrchestrationRoutingService;

impl OrchestrationRoutingService {
    pub fn new() -> Self {
        Self
    }

    pub fn resolve_route(
        &self,
        runtime: &OrchestrationRuntimeSpec,
        claws: &[OrchestrationClawRouteSpec],
        request: &OrchestrationRequest,
    ) -> Result<OrchestrationRouteSelection> {
        let mut runtime = runtime.clone();
        self.apply_request_overrides(&mut runtime.autonomy, &request.overrides);
        self.validate_request_overrides(&runtime.autonomy)?;

        let task_assignment = request.task_id.as_deref().and_then(|task_id| {
            runtime
                .task_assignments
                .iter()
                .find(|(candidate, _)| candidate == task_id)
                .map(|(_, claw_id)| claw_id.clone())
        });
        let category_assignment = request.category.as_deref().and_then(|category| {
            runtime
                .category_assignments
                .iter()
                .find(|(candidate, _)| candidate == category)
                .map(|(_, claw_id)| claw_id.clone())
        });

        let (selected_claw_id, route_source) = if let Some(claw_id) = request.claw_id.as_ref() {
            (claw_id.clone(), "explicit_claw".to_string())
        } else if let Some(claw_id) = task_assignment {
            (claw_id, "task_assignment".to_string())
        } else if let Some(claw_id) = category_assignment {
            (claw_id, "category_assignment".to_string())
        } else if runtime.mode == "orchestrated" && request.mode != "direct" {
            (
                runtime
                    .orchestrator_claw_id
                    .clone()
                    .or_else(|| runtime.default_claw_id.clone())
                    .ok_or_else(|| {
                        Error::Config("No orchestrator/default claw configured".to_string())
                    })?,
                "orchestrated_default".to_string(),
            )
        } else {
            (
                runtime
                    .default_claw_id
                    .clone()
                    .or_else(|| self.first_enabled_claw_id(claws))
                    .ok_or_else(|| Error::Config("No default claw configured".to_string()))?,
                "default_claw".to_string(),
            )
        };

        let selected_claw = claws
            .iter()
            .find(|candidate| candidate.id == selected_claw_id)
            .ok_or_else(|| Error::Config(format!("Unknown claw '{}'", selected_claw_id)))?;
        let mut available_workers = claws
            .iter()
            .filter(|candidate| candidate.enabled && candidate.id != selected_claw.id)
            .map(|candidate| candidate.id.clone())
            .collect::<Vec<_>>();
        available_workers.sort();

        Ok(OrchestrationRouteSelection {
            execution_mode: runtime.mode,
            route_source,
            selected_claw_id,
            selected_claw_role: selected_claw.role.clone(),
            available_workers,
            autonomy: runtime.autonomy,
            allow_shared_context: runtime.allow_shared_context,
            isolation_mode: runtime.isolation_mode,
        })
    }

    pub fn intervention_transition(
        &self,
        action: &str,
        reason: Option<&str>,
        rollback_reference: Option<&str>,
    ) -> Result<ActiveRunInterventionTransition> {
        let transition = match action {
            "pause" => ActiveRunInterventionTransition {
                stage: "supervision_pause".to_string(),
                status: "pause_requested".to_string(),
                note: reason
                    .map(|value| format!("pause requested by operator: {value}"))
                    .unwrap_or_else(|| "pause requested by operator".to_string()),
                resulting_state: "pause_requested".to_string(),
                pause_requested: true,
                kill_requested: false,
                lifecycle: OrchestrationLifecycleSummary {
                    state: "pause_requested".to_string(),
                    intervention_required: false,
                    escalation_requested: false,
                    rollback_requested: false,
                    rollback_reference: None,
                },
            },
            "resume" => ActiveRunInterventionTransition {
                stage: "supervision_resume".to_string(),
                status: "running".to_string(),
                note: reason
                    .map(|value| format!("resumed by operator: {value}"))
                    .unwrap_or_else(|| "resumed by operator".to_string()),
                resulting_state: "running".to_string(),
                pause_requested: false,
                kill_requested: false,
                lifecycle: OrchestrationLifecycleSummary {
                    state: "running".to_string(),
                    intervention_required: false,
                    escalation_requested: false,
                    rollback_requested: false,
                    rollback_reference: None,
                },
            },
            "kill" => ActiveRunInterventionTransition {
                stage: "supervision_kill".to_string(),
                status: "kill_requested".to_string(),
                note: reason
                    .map(|value| format!("kill requested by operator: {value}"))
                    .unwrap_or_else(|| "kill requested by operator".to_string()),
                resulting_state: "kill_requested".to_string(),
                pause_requested: false,
                kill_requested: true,
                lifecycle: OrchestrationLifecycleSummary {
                    state: "kill_requested".to_string(),
                    intervention_required: true,
                    escalation_requested: false,
                    rollback_requested: false,
                    rollback_reference: None,
                },
            },
            "escalate" => ActiveRunInterventionTransition {
                stage: "supervision_escalate".to_string(),
                status: "escalated".to_string(),
                note: reason
                    .map(|value| format!("escalated for operator review: {value}"))
                    .unwrap_or_else(|| "escalated for operator review".to_string()),
                resulting_state: "escalated".to_string(),
                pause_requested: true,
                kill_requested: false,
                lifecycle: OrchestrationLifecycleSummary {
                    state: "escalated".to_string(),
                    intervention_required: true,
                    escalation_requested: true,
                    rollback_requested: false,
                    rollback_reference: None,
                },
            },
            "rollback" => ActiveRunInterventionTransition {
                stage: "supervision_rollback".to_string(),
                status: "rollback_requested".to_string(),
                note: reason
                    .map(|value| format!("rollback requested by operator: {value}"))
                    .unwrap_or_else(|| "rollback requested by operator".to_string()),
                resulting_state: "rollback_requested".to_string(),
                pause_requested: true,
                kill_requested: true,
                lifecycle: OrchestrationLifecycleSummary {
                    state: "rollback_requested".to_string(),
                    intervention_required: true,
                    escalation_requested: false,
                    rollback_requested: true,
                    rollback_reference: rollback_reference.map(ToString::to_string),
                },
            },
            other => {
                return Err(Error::Config(format!(
                    "unsupported orchestration lifecycle action '{other}'"
                )));
            }
        };
        Ok(transition)
    }

    fn apply_request_overrides(
        &self,
        autonomy: &mut OrchestrationAutonomyPolicy,
        overrides: &OrchestrationRequestOverrides,
    ) {
        if let Some(level) = overrides.autonomy_level.as_deref() {
            autonomy.autonomy_level = level.to_string();
            autonomy.yolo_mode = level == "yolo";
        }
        if let Some(max_delegations) = overrides.max_delegations {
            autonomy.max_delegations = max_delegations.max(1);
        }
        if let Some(max_iterations) = overrides.max_iterations {
            autonomy.max_iterations = max_iterations.max(1);
        }
        if let Some(max_runtime_secs) = overrides.max_runtime_secs {
            autonomy.max_runtime_secs = max_runtime_secs.max(1);
        }
        if let Some(approval_policy) = overrides.approval_policy.as_deref() {
            autonomy.approval_policy = approval_policy.to_string();
        }
    }

    fn validate_request_overrides(&self, autonomy: &OrchestrationAutonomyPolicy) -> Result<()> {
        match autonomy.autonomy_level.as_str() {
            "assisted" | "supervised" | "managed" | "autonomous" | "yolo" => {}
            other => return Err(Error::Config(format!("invalid autonomy level '{}'", other))),
        }
        match autonomy.approval_policy.as_str() {
            "none" | "side_effects" | "always" => {}
            other => {
                return Err(Error::Config(format!(
                    "invalid approval policy '{}'",
                    other
                )));
            }
        }
        Ok(())
    }

    fn first_enabled_claw_id(&self, claws: &[OrchestrationClawRouteSpec]) -> Option<String> {
        claws
            .iter()
            .find(|candidate| candidate.enabled)
            .map(|candidate| candidate.id.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn autonomy() -> OrchestrationAutonomyPolicy {
        OrchestrationAutonomyPolicy {
            autonomy_level: "supervised".to_string(),
            yolo_mode: false,
            steering_enabled: true,
            decision_learning_enabled: true,
            critic_enabled: true,
            max_delegations: 3,
            max_iterations: 6,
            max_runtime_secs: 120,
            max_lesson_hints: 5,
            approval_policy: "always".to_string(),
        }
    }

    fn runtime() -> OrchestrationRuntimeSpec {
        OrchestrationRuntimeSpec {
            mode: "orchestrated".to_string(),
            default_claw_id: Some("main".to_string()),
            orchestrator_claw_id: Some("orchestrator".to_string()),
            task_assignments: vec![("task-1".to_string(), "main".to_string())],
            category_assignments: vec![("code".to_string(), "main".to_string())],
            allow_shared_context: false,
            isolation_mode: "strict".to_string(),
            autonomy: autonomy(),
        }
    }

    fn claws() -> Vec<OrchestrationClawRouteSpec> {
        vec![
            OrchestrationClawRouteSpec {
                id: "main".to_string(),
                role: "primary".to_string(),
                enabled: true,
            },
            OrchestrationClawRouteSpec {
                id: "orchestrator".to_string(),
                role: "orchestrator".to_string(),
                enabled: true,
            },
        ]
    }

    #[test]
    fn resolves_task_assignment_before_default() {
        let service = OrchestrationRoutingService::new();
        let selection = service
            .resolve_route(
                &runtime(),
                &claws(),
                &OrchestrationRequest {
                    prompt: "test".to_string(),
                    task_id: Some("task-1".to_string()),
                    category: Some("code".to_string()),
                    claw_id: None,
                    mode: "auto".to_string(),
                    overrides: OrchestrationRequestOverrides::default(),
                },
            )
            .unwrap();
        assert_eq!(selection.selected_claw_id, "main");
        assert_eq!(selection.route_source, "task_assignment");
    }

    #[test]
    fn orchestrated_mode_prefers_orchestrator() {
        let service = OrchestrationRoutingService::new();
        let selection = service
            .resolve_route(
                &runtime(),
                &claws(),
                &OrchestrationRequest {
                    prompt: "test".to_string(),
                    task_id: None,
                    category: None,
                    claw_id: None,
                    mode: "auto".to_string(),
                    overrides: OrchestrationRequestOverrides::default(),
                },
            )
            .unwrap();
        assert_eq!(selection.selected_claw_id, "orchestrator");
        assert_eq!(selection.selected_claw_role, "orchestrator");
    }

    #[test]
    fn request_overrides_update_autonomy_bounds() {
        let service = OrchestrationRoutingService::new();
        let selection = service
            .resolve_route(
                &runtime(),
                &claws(),
                &OrchestrationRequest {
                    prompt: "test".to_string(),
                    task_id: None,
                    category: None,
                    claw_id: Some("main".to_string()),
                    mode: "auto".to_string(),
                    overrides: OrchestrationRequestOverrides {
                        autonomy_level: Some("managed".to_string()),
                        max_delegations: Some(0),
                        max_iterations: Some(9),
                        max_runtime_secs: Some(45),
                        approval_policy: Some("side_effects".to_string()),
                        ..OrchestrationRequestOverrides::default()
                    },
                },
            )
            .unwrap();
        assert_eq!(selection.autonomy.autonomy_level, "managed");
        assert_eq!(selection.autonomy.max_delegations, 1);
        assert_eq!(selection.autonomy.max_iterations, 9);
        assert_eq!(selection.autonomy.max_runtime_secs, 45);
        assert_eq!(selection.autonomy.approval_policy, "side_effects");
    }

    #[test]
    fn intervention_transition_marks_rollback_as_terminal() {
        let service = OrchestrationRoutingService::new();
        let transition = service
            .intervention_transition("rollback", Some("unsafe"), Some("receipt-17.json"))
            .unwrap();
        assert_eq!(transition.stage, "supervision_rollback");
        assert!(transition.pause_requested);
        assert!(transition.kill_requested);
        assert!(transition.lifecycle.rollback_requested);
        assert_eq!(
            transition.lifecycle.rollback_reference.as_deref(),
            Some("receipt-17.json")
        );
    }
}
