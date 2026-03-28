use openrustclaw_core::error::{Error, Result};
use serde::{Deserialize, Serialize};

pub const MODE_SOLO: &str = "solo";
pub const MODE_TEAM: &str = "team";
pub const MODE_COMPANY: &str = "company";
pub const MODE_ENTERPRISE: &str = "enterprise";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SelfHostedProductTransitionEvent {
    pub created_at: String,
    pub from_mode: String,
    pub to_mode: String,
    pub direction: String,
    pub actor: String,
    #[serde(default)]
    pub reason: Option<String>,
    pub via: String,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SelfHostedProductModeState {
    pub manifest_path: String,
    pub events_path: String,
    pub explicit_mode_selected: bool,
    pub self_hosted: bool,
    pub open_source: bool,
    pub mode: String,
    pub onboarding_path: String,
    #[serde(default)]
    pub current_warnings: Vec<String>,
    #[serde(default)]
    pub recent_transitions: Vec<SelfHostedProductTransitionEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SelfHostedProductModeReport {
    pub status: String,
    pub detail: String,
    pub manifest_path: String,
    pub events_path: String,
    pub explicit_mode_selected: bool,
    pub self_hosted: bool,
    pub open_source: bool,
    pub mode: String,
    pub mode_label: String,
    pub onboarding_path: String,
    pub operator_model: String,
    pub recommended_runtime_mode: String,
    pub multi_user: bool,
    pub enterprise_controls_expected: bool,
    pub transition_targets: Vec<String>,
    pub upgrade_targets: Vec<String>,
    pub downgrade_targets: Vec<String>,
    pub current_warnings: Vec<String>,
    pub recent_transitions: Vec<SelfHostedProductTransitionEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SelfHostedProductModeTransitionRequest {
    pub target_mode: String,
    pub actor: String,
    #[serde(default)]
    pub reason: Option<String>,
    pub via: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SelfHostedModeDescriptor {
    mode: &'static str,
    label: &'static str,
    operator_model: &'static str,
    recommended_runtime_mode: &'static str,
    detail: &'static str,
    multi_user: bool,
    enterprise_controls_expected: bool,
    transition_targets: &'static [&'static str],
}

fn normalize_mode(mode: &str) -> Result<String> {
    let normalized = mode.trim().to_ascii_lowercase();
    match normalized.as_str() {
        MODE_SOLO | MODE_TEAM | MODE_COMPANY | MODE_ENTERPRISE => Ok(normalized),
        _ => Err(Error::Internal(format!(
            "unsupported product mode '{}'; expected one of: solo, team, company, enterprise",
            mode
        ))),
    }
}

fn descriptor_for(mode: &str) -> Result<SelfHostedModeDescriptor> {
    let descriptor = match normalize_mode(mode)?.as_str() {
        MODE_SOLO => SelfHostedModeDescriptor {
            mode: MODE_SOLO,
            label: "Solo",
            operator_model: "One primary operator on one self-hosted workspace",
            recommended_runtime_mode: "solo_claw",
            detail: "Solo mode keeps OpenRustClaw lightweight for one operator while preserving the full self-hosted open-source runtime.",
            multi_user: false,
            enterprise_controls_expected: false,
            transition_targets: &[MODE_TEAM, MODE_COMPANY, MODE_ENTERPRISE],
        },
        MODE_TEAM => SelfHostedModeDescriptor {
            mode: MODE_TEAM,
            label: "Multi-User Team",
            operator_model: "A small team shares one self-hosted deployment",
            recommended_runtime_mode: "task_assigned",
            detail: "Team mode keeps the product self-hosted and open-source while preparing the workspace for multiple operators and shared channels.",
            multi_user: true,
            enterprise_controls_expected: false,
            transition_targets: &[MODE_SOLO, MODE_COMPANY, MODE_ENTERPRISE],
        },
        MODE_COMPANY => SelfHostedModeDescriptor {
            mode: MODE_COMPANY,
            label: "Company",
            operator_model: "An operator-managed company deployment with stronger operational defaults",
            recommended_runtime_mode: "orchestrated",
            detail: "Company mode assumes a broader internal deployment, stronger operator practices, and a clearer path into governance without claiming full enterprise controls by default.",
            multi_user: true,
            enterprise_controls_expected: true,
            transition_targets: &[MODE_TEAM, MODE_ENTERPRISE],
        },
        MODE_ENTERPRISE => SelfHostedModeDescriptor {
            mode: MODE_ENTERPRISE,
            label: "Enterprise",
            operator_model: "A governed enterprise deployment with explicit access, policy, and audit expectations",
            recommended_runtime_mode: "orchestrated",
            detail: "Enterprise mode keeps the product self-hosted and open-source while signaling the strongest operator, policy, audit, and governance expectations in the current shipped runtime.",
            multi_user: true,
            enterprise_controls_expected: true,
            transition_targets: &[MODE_COMPANY, MODE_TEAM],
        },
        _ => unreachable!("validated by normalize_mode"),
    };
    Ok(descriptor)
}

fn upgrade_targets(descriptor: SelfHostedModeDescriptor) -> Vec<String> {
    descriptor
        .transition_targets
        .iter()
        .copied()
        .filter(|value| value != &descriptor.mode)
        .filter(|value| match *value {
            MODE_TEAM => descriptor.mode == MODE_SOLO,
            MODE_COMPANY => descriptor.mode == MODE_SOLO || descriptor.mode == MODE_TEAM,
            MODE_ENTERPRISE => descriptor.mode != MODE_ENTERPRISE,
            _ => false,
        })
        .map(str::to_string)
        .collect()
}

fn downgrade_targets(
    descriptor: SelfHostedModeDescriptor,
    upgrade_targets: &[String],
) -> Vec<String> {
    descriptor
        .transition_targets
        .iter()
        .copied()
        .filter(|value| !upgrade_targets.iter().any(|entry| entry == value))
        .map(str::to_string)
        .collect()
}

pub trait SelfHostedProductModeSource {
    fn load_self_hosted_product_mode_state(&self) -> Result<SelfHostedProductModeState>;
}

pub trait SelfHostedProductModeTransitionExecutor {
    fn transition_self_hosted_product_mode(
        &self,
        request: SelfHostedProductModeTransitionRequest,
    ) -> Result<()>;
}

pub struct SelfHostedProductModeService<S> {
    source: S,
}

impl<S> SelfHostedProductModeService<S> {
    pub fn new(source: S) -> Self {
        Self { source }
    }
}

impl<S> SelfHostedProductModeService<S>
where
    S: SelfHostedProductModeSource,
{
    pub fn report(&self) -> Result<SelfHostedProductModeReport> {
        report_from_state(self.source.load_self_hosted_product_mode_state()?)
    }
}

pub struct SelfHostedProductModeControlService<S> {
    source: S,
}

impl<S> SelfHostedProductModeControlService<S> {
    pub fn new(source: S) -> Self {
        Self { source }
    }
}

impl<S> SelfHostedProductModeControlService<S>
where
    S: SelfHostedProductModeSource + SelfHostedProductModeTransitionExecutor,
{
    pub fn transition_and_report(
        &self,
        request: SelfHostedProductModeTransitionRequest,
    ) -> Result<SelfHostedProductModeReport> {
        self.source.transition_self_hosted_product_mode(request)?;
        report_from_state(self.source.load_self_hosted_product_mode_state()?)
    }
}

fn report_from_state(state: SelfHostedProductModeState) -> Result<SelfHostedProductModeReport> {
    let descriptor = descriptor_for(&state.mode)?;
    let upgrade_targets = upgrade_targets(descriptor);
    let downgrade_targets = downgrade_targets(descriptor, &upgrade_targets);
    let detail = if state.explicit_mode_selected {
        format!(
            "OpenRustClaw is configured as a {} self-hosted open-source deployment. {} The current onboarding path is `{}`, the recommended runtime execution mode is `{}`, and the next valid product-mode transitions are {}.",
            descriptor.label,
            descriptor.detail,
            state.onboarding_path,
            descriptor.recommended_runtime_mode,
            descriptor.transition_targets.join(", ")
        )
    } else {
        format!(
            "No explicit product mode is saved yet, so OpenRustClaw currently reads as the default {} self-hosted open-source deployment. {} Run onboarding to lock in a mode-specific path before broadening the operator surface.",
            descriptor.label.to_ascii_lowercase(),
            descriptor.detail
        )
    };

    Ok(SelfHostedProductModeReport {
        status: if state.explicit_mode_selected {
            "ok".to_string()
        } else {
            "implicit_default".to_string()
        },
        detail,
        manifest_path: state.manifest_path,
        events_path: state.events_path,
        explicit_mode_selected: state.explicit_mode_selected,
        self_hosted: state.self_hosted,
        open_source: state.open_source,
        mode: descriptor.mode.to_string(),
        mode_label: descriptor.label.to_string(),
        onboarding_path: state.onboarding_path,
        operator_model: descriptor.operator_model.to_string(),
        recommended_runtime_mode: descriptor.recommended_runtime_mode.to_string(),
        multi_user: descriptor.multi_user,
        enterprise_controls_expected: descriptor.enterprise_controls_expected,
        transition_targets: descriptor
            .transition_targets
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        upgrade_targets,
        downgrade_targets,
        current_warnings: state.current_warnings,
        recent_transitions: state.recent_transitions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    struct StubSource {
        state: SelfHostedProductModeState,
    }

    impl SelfHostedProductModeSource for StubSource {
        fn load_self_hosted_product_mode_state(&self) -> Result<SelfHostedProductModeState> {
            Ok(self.state.clone())
        }
    }

    struct MutableStubSource {
        state: RefCell<SelfHostedProductModeState>,
    }

    impl SelfHostedProductModeSource for MutableStubSource {
        fn load_self_hosted_product_mode_state(&self) -> Result<SelfHostedProductModeState> {
            Ok(self.state.borrow().clone())
        }
    }

    impl SelfHostedProductModeTransitionExecutor for MutableStubSource {
        fn transition_self_hosted_product_mode(
            &self,
            request: SelfHostedProductModeTransitionRequest,
        ) -> Result<()> {
            let mut state = self.state.borrow_mut();
            let target_mode = normalize_mode(&request.target_mode)?;
            state.explicit_mode_selected = true;
            state.mode = target_mode.clone();
            state.onboarding_path = format!("{target_mode}_path");
            state.recent_transitions.insert(
                0,
                SelfHostedProductTransitionEvent {
                    created_at: "2026-03-28T12:00:00Z".to_string(),
                    from_mode: MODE_TEAM.to_string(),
                    to_mode: target_mode,
                    direction: "upgrade".to_string(),
                    actor: request.actor,
                    reason: request.reason,
                    via: request.via,
                    warnings: Vec::new(),
                },
            );
            Ok(())
        }
    }

    #[test]
    fn report_defaults_to_implicit_solo_contract() -> Result<()> {
        let service = SelfHostedProductModeService::new(StubSource {
            state: SelfHostedProductModeState {
                manifest_path: ".claw/control/self-hosted/product-mode.json".to_string(),
                events_path: ".claw/control/self-hosted/product-mode-events.jsonl".to_string(),
                explicit_mode_selected: false,
                self_hosted: true,
                open_source: true,
                mode: MODE_SOLO.to_string(),
                onboarding_path: "solo_starter".to_string(),
                current_warnings: Vec::new(),
                recent_transitions: Vec::new(),
            },
        });

        let report = service.report()?;
        assert_eq!(report.status, "implicit_default");
        assert_eq!(report.mode, MODE_SOLO);
        assert!(
            report
                .transition_targets
                .iter()
                .any(|entry| entry == MODE_TEAM)
        );
        assert_eq!(report.recommended_runtime_mode, "solo_claw");
        Ok(())
    }

    #[test]
    fn report_preserves_company_mode_state() -> Result<()> {
        let service = SelfHostedProductModeService::new(StubSource {
            state: SelfHostedProductModeState {
                manifest_path: ".claw/control/self-hosted/product-mode.json".to_string(),
                events_path: ".claw/control/self-hosted/product-mode-events.jsonl".to_string(),
                explicit_mode_selected: true,
                self_hosted: true,
                open_source: true,
                mode: MODE_COMPANY.to_string(),
                onboarding_path: "company_ops_setup".to_string(),
                current_warnings: vec!["warning".to_string()],
                recent_transitions: vec![SelfHostedProductTransitionEvent {
                    created_at: "2026-03-28T00:00:00Z".to_string(),
                    from_mode: MODE_TEAM.to_string(),
                    to_mode: MODE_COMPANY.to_string(),
                    direction: "upgrade".to_string(),
                    actor: "operator-1".to_string(),
                    reason: Some("team grew".to_string()),
                    via: "test".to_string(),
                    warnings: Vec::new(),
                }],
            },
        });

        let report = service.report()?;
        assert_eq!(report.status, "ok");
        assert_eq!(report.mode_label, "Company");
        assert!(report.enterprise_controls_expected);
        assert!(report.multi_user);
        assert_eq!(report.recent_transitions.len(), 1);
        assert_eq!(report.upgrade_targets, vec![MODE_ENTERPRISE.to_string()]);
        Ok(())
    }

    #[test]
    fn control_service_transitions_and_returns_updated_report() -> Result<()> {
        let service = SelfHostedProductModeControlService::new(MutableStubSource {
            state: RefCell::new(SelfHostedProductModeState {
                manifest_path: ".claw/control/self-hosted/product-mode.json".to_string(),
                events_path: ".claw/control/self-hosted/product-mode-events.jsonl".to_string(),
                explicit_mode_selected: true,
                self_hosted: true,
                open_source: true,
                mode: MODE_TEAM.to_string(),
                onboarding_path: "team_path".to_string(),
                current_warnings: Vec::new(),
                recent_transitions: Vec::new(),
            }),
        });

        let report = service.transition_and_report(SelfHostedProductModeTransitionRequest {
            target_mode: MODE_COMPANY.to_string(),
            actor: "operator-1".to_string(),
            reason: Some("team grew".to_string()),
            via: "control_api".to_string(),
        })?;

        assert_eq!(report.mode, MODE_COMPANY);
        assert_eq!(report.status, "ok");
        assert_eq!(report.onboarding_path, "company_path");
        assert_eq!(report.recent_transitions.len(), 1);
        assert_eq!(report.recent_transitions[0].actor, "operator-1");
        Ok(())
    }
}
