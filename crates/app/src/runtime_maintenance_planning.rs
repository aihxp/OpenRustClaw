use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeUpgradePlanningRequest {
    pub generated_at: String,
    pub config_path: String,
    pub default_or_fallback_healthy: bool,
    pub service_manager_supported: bool,
    pub service_installed: bool,
    pub restart_command: Option<String>,
    pub lock_active: bool,
    pub reload_restart_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeUpgradePlanningReport {
    pub backup_command: String,
    pub log_rotation_command: String,
    pub migrate_config_command: String,
    pub ready: bool,
    pub blockers: Vec<String>,
    pub steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeSelfUpdatePlanningRequest {
    pub generated_at: String,
    pub config_path: String,
    pub current_executable: String,
    pub artifact_path: String,
    pub recommended_rollback_path: String,
    pub artifact_exists: bool,
    pub artifact_is_file: bool,
    pub artifact_executable: bool,
    pub current_executable_matches_artifact: bool,
    pub service_installed: bool,
    pub restart_command: Option<String>,
    pub lock_active: bool,
    #[serde(default)]
    pub inherited_upgrade_blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeSelfUpdatePlanningReport {
    pub backup_command: String,
    pub ready: bool,
    pub blockers: Vec<String>,
    pub steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeRollbackPlanningRequest {
    pub generated_at: String,
    pub config_path: String,
    pub current_executable: String,
    pub artifact_path: String,
    pub artifact_exists: bool,
    pub artifact_is_file: bool,
    pub artifact_executable: bool,
    pub current_executable_matches_artifact: bool,
    pub service_installed: bool,
    pub restart_command: Option<String>,
    pub lock_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeRollbackPlanningReport {
    pub backup_command: String,
    pub ready: bool,
    pub blockers: Vec<String>,
    pub steps: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct RuntimeMaintenancePlanningService;

impl RuntimeMaintenancePlanningService {
    pub fn new() -> Self {
        Self
    }

    pub fn build_upgrade_plan(
        &self,
        request: RuntimeUpgradePlanningRequest,
    ) -> RuntimeUpgradePlanningReport {
        let mut blockers = Vec::new();
        if request.lock_active {
            blockers.push(
                "runtime lock is currently active; schedule the upgrade around the running process or stop it first"
                    .to_string(),
            );
        }
        if !request.default_or_fallback_healthy {
            blockers.push(
                "no healthy default or fallback control-plane provider is currently available"
                    .to_string(),
            );
        }
        if !request.service_manager_supported {
            blockers.push(
                "no supported host user service manager is available on this host".to_string(),
            );
        }

        let backup_command = "openrustclaw runtime backup".to_string();
        let log_rotation_command =
            "openrustclaw runtime services rotate-logs --keep 7 --max-bytes 10485760".to_string();
        let migrate_config_command = format!(
            "openrustclaw runtime migrate-config --config {} --apply",
            request.config_path
        );
        let mut steps = vec![
            format!(
                "Take a workspace snapshot with `{}` before changing config or binaries.",
                backup_command
            ),
            format!(
                "Rotate and archive the runtime log with `{}` before restart windows.",
                log_rotation_command
            ),
            format!(
                "Run `openrustclaw runtime migrate-config --config {} --apply` to normalize any remaining legacy config keys.",
                request.config_path
            ),
        ];
        if request.reload_restart_required {
            steps.push("The current reload plan reports restart-required changes; prefer a controlled restart rather than live rebind.".to_string());
        } else {
            steps.push("The current reload plan is live-safe; apply non-disruptive config changes before the maintenance restart if needed.".to_string());
        }
        if request.service_installed {
            if let Some(restart_command) = request.restart_command.as_ref() {
                steps.push(format!(
                    "Use the installed user service to restart cleanly after the upgrade with `{}`.",
                    restart_command
                ));
            } else {
                steps.push(
                    "Use the installed user service to restart cleanly after the upgrade."
                        .to_string(),
                );
            }
        } else {
            steps.push("Consider `openrustclaw runtime services install --config ...` if this workspace should restart under a managed user service.".to_string());
        }

        RuntimeUpgradePlanningReport {
            backup_command,
            log_rotation_command,
            migrate_config_command,
            ready: blockers.is_empty(),
            blockers,
            steps,
        }
    }

    pub fn build_self_update_plan(
        &self,
        request: RuntimeSelfUpdatePlanningRequest,
    ) -> RuntimeSelfUpdatePlanningReport {
        let mut blockers = Vec::new();
        if request.lock_active {
            blockers.push(
                "runtime lock is active; stop or drain the running process before replacing the binary"
                    .to_string(),
            );
        }
        if !request.artifact_exists {
            blockers.push(format!(
                "candidate artifact '{}' does not exist",
                request.artifact_path
            ));
        } else if !request.artifact_is_file {
            blockers.push(format!(
                "candidate artifact '{}' is not a regular file",
                request.artifact_path
            ));
        } else if !request.artifact_executable {
            blockers.push(format!(
                "candidate artifact '{}' is not marked executable",
                request.artifact_path
            ));
        }
        if request.current_executable_matches_artifact {
            blockers.push(
                "candidate artifact resolves to the currently running executable".to_string(),
            );
        }
        blockers.extend(request.inherited_upgrade_blockers.iter().cloned());

        let backup_command = "openrustclaw runtime backup".to_string();
        let mut steps = vec![
            format!(
                "Take a workspace snapshot with `{}` before replacing the binary.",
                backup_command
            ),
            format!(
                "Copy the current executable to '{}' as the rollback reference.",
                request.recommended_rollback_path
            ),
            format!(
                "Validate the candidate artifact at '{}' and then replace the installed binary at '{}'.",
                request.artifact_path, request.current_executable
            ),
        ];
        if let Some(restart_command) = request.restart_command.as_ref() {
            steps.push(format!(
                "Restart the managed runtime with `{}` after the binary swap.",
                restart_command
            ));
        } else if request.service_installed {
            steps.push("Restart the managed runtime after the binary swap.".to_string());
        } else {
            steps.push(
                "Restart the runtime manually after the binary swap because no managed host service is installed."
                    .to_string(),
            );
        }
        steps.push(format!(
            "If the new binary is unhealthy, roll back with `openrustclaw runtime rollback-plan --config {} --artifact {}`.",
            request.config_path, request.recommended_rollback_path
        ));

        RuntimeSelfUpdatePlanningReport {
            backup_command,
            ready: blockers.is_empty(),
            blockers,
            steps,
        }
    }

    pub fn build_rollback_plan(
        &self,
        request: RuntimeRollbackPlanningRequest,
    ) -> RuntimeRollbackPlanningReport {
        let mut blockers = Vec::new();
        if request.lock_active {
            blockers.push(
                "runtime lock is active; stop or drain the running process before rolling the binary back"
                    .to_string(),
            );
        }
        if !request.artifact_exists {
            blockers.push(format!(
                "rollback artifact '{}' does not exist",
                request.artifact_path
            ));
        } else if !request.artifact_is_file {
            blockers.push(format!(
                "rollback artifact '{}' is not a regular file",
                request.artifact_path
            ));
        } else if !request.artifact_executable {
            blockers.push(format!(
                "rollback artifact '{}' is not marked executable",
                request.artifact_path
            ));
        }
        if request.current_executable_matches_artifact {
            blockers
                .push("rollback artifact resolves to the currently running executable".to_string());
        }

        let backup_command = "openrustclaw runtime backup".to_string();
        let mut steps = vec![
            format!(
                "Take a fresh workspace snapshot with `{}` before restoring the prior binary.",
                backup_command
            ),
            format!(
                "Replace the installed binary at '{}' with the rollback artifact at '{}'.",
                request.current_executable, request.artifact_path
            ),
        ];
        if let Some(restart_command) = request.restart_command.as_ref() {
            steps.push(format!(
                "Restart the managed runtime with `{}` after restoring the binary.",
                restart_command
            ));
        } else if request.service_installed {
            steps.push("Restart the managed runtime after restoring the binary.".to_string());
        } else {
            steps.push(
                "Restart the runtime manually after restoring the binary because no managed host service is installed."
                    .to_string(),
            );
        }
        steps.push(format!(
            "Re-run `openrustclaw runtime upgrade-plan --config {}` and `openrustclaw runtime health` after rollback verification.",
            request.config_path
        ));

        RuntimeRollbackPlanningReport {
            backup_command,
            ready: blockers.is_empty(),
            blockers,
            steps,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upgrade_plan_reports_blockers_and_steps() {
        let report = RuntimeMaintenancePlanningService::new().build_upgrade_plan(
            RuntimeUpgradePlanningRequest {
                generated_at: "2026-03-28T00:00:00Z".to_string(),
                config_path: "config/default.toml".to_string(),
                default_or_fallback_healthy: false,
                service_manager_supported: true,
                service_installed: false,
                restart_command: None,
                lock_active: true,
                reload_restart_required: true,
            },
        );

        assert!(!report.ready);
        assert_eq!(report.blockers.len(), 2);
        assert!(
            report
                .steps
                .iter()
                .any(|step| step.contains("runtime migrate-config"))
        );
    }

    #[test]
    fn self_update_plan_inherits_upgrade_blockers() {
        let report = RuntimeMaintenancePlanningService::new().build_self_update_plan(
            RuntimeSelfUpdatePlanningRequest {
                generated_at: "2026-03-28T00:00:00Z".to_string(),
                config_path: "config/default.toml".to_string(),
                current_executable: "/tmp/openrustclaw".to_string(),
                artifact_path: "/tmp/openrustclaw-next".to_string(),
                recommended_rollback_path: "/tmp/rollback".to_string(),
                artifact_exists: true,
                artifact_is_file: true,
                artifact_executable: true,
                current_executable_matches_artifact: false,
                service_installed: false,
                restart_command: None,
                lock_active: false,
                inherited_upgrade_blockers: vec!["upstream blocker".to_string()],
            },
        );

        assert!(!report.ready);
        assert_eq!(report.blockers, vec!["upstream blocker".to_string()]);
        assert!(
            report
                .steps
                .iter()
                .any(|step| step.contains("rollback-plan"))
        );
    }
}
