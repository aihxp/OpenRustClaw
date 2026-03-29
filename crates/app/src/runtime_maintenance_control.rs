use crate::greenfield_progress::{GreenfieldProgressReport, GreenfieldSeam, GreenfieldSeamStatus};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuntimeMaintenanceRouteHints {
    pub maintenance_path: String,
    pub upgrade_plan_path: String,
    pub self_update_plan_path: String,
    pub rollback_plan_path: String,
    pub artifact_query_parameter: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuntimeMaintenanceControlReport {
    pub status: String,
    pub detail: String,
    pub greenfield_progress: GreenfieldProgressReport,
    pub remaining_queue: Vec<GreenfieldSeam>,
    pub route_hints: RuntimeMaintenanceRouteHints,
}

#[derive(Debug, Clone, Default)]
pub struct RuntimeMaintenanceControlService;

impl RuntimeMaintenanceControlService {
    pub fn new() -> Self {
        Self
    }

    pub fn report(
        &self,
        greenfield_progress: GreenfieldProgressReport,
    ) -> RuntimeMaintenanceControlReport {
        let remaining_queue = greenfield_progress
            .seams
            .iter()
            .filter(|seam| seam.status == GreenfieldSeamStatus::Remaining)
            .cloned()
            .collect::<Vec<_>>();

        let detail = if let Some(next_seam) = remaining_queue.first() {
            format!(
                "Application boundary migration is {}% complete ({} of {} ranked seams migrated). Next remaining seam: {}.",
                greenfield_progress.completion_percent,
                greenfield_progress.completed_seams,
                greenfield_progress.total_seams,
                next_seam.name
            )
        } else {
            format!(
                "Application boundary migration is {}% complete and the current ranked seam inventory is retired at 18/18. Future follow-on work requires an explicit new canonical queue.",
                greenfield_progress.completion_percent
            )
        };

        RuntimeMaintenanceControlReport {
            status: "ok".to_string(),
            detail,
            greenfield_progress,
            remaining_queue,
            route_hints: RuntimeMaintenanceRouteHints {
                maintenance_path: "/control/runtime/maintenance".to_string(),
                upgrade_plan_path: "/control/runtime/upgrade-plan".to_string(),
                self_update_plan_path: "/control/runtime/self-update-plan".to_string(),
                rollback_plan_path: "/control/runtime/rollback-plan".to_string(),
                artifact_query_parameter: "artifact".to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RuntimeMaintenanceControlService;
    use crate::greenfield_progress::GreenfieldProgressService;

    #[test]
    fn report_surfaces_progress_and_remaining_queue() {
        let progress = GreenfieldProgressService::new().report("2026-03-28T17:00:00Z".to_string());
        let report = RuntimeMaintenanceControlService::new().report(progress);

        assert_eq!(report.status, "ok");
        assert_eq!(report.greenfield_progress.completion_percent, 100);
        assert_eq!(report.greenfield_progress.ledger_status, "complete");
        assert_eq!(
            report.greenfield_progress.queue_decision,
            "retire_current_ranked_inventory"
        );
        assert!(report.remaining_queue.is_empty());
        assert_eq!(
            report.route_hints.maintenance_path,
            "/control/runtime/maintenance"
        );
        assert_eq!(report.route_hints.artifact_query_parameter, "artifact");
    }
}
