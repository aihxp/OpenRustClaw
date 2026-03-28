use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GreenfieldSeamStatus {
    Migrated,
    Remaining,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GreenfieldSeam {
    pub rank: u32,
    pub area: String,
    pub name: String,
    pub source_phase: Option<String>,
    pub status: GreenfieldSeamStatus,
    pub rationale: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GreenfieldProgressReport {
    pub generated_at: String,
    pub baseline_name: String,
    pub baseline_note: String,
    pub ledger_status: String,
    pub queue_decision: String,
    pub follow_on_note: String,
    pub completed_seams: usize,
    pub remaining_seams: usize,
    pub total_seams: usize,
    pub completion_percent: u8,
    pub seams: Vec<GreenfieldSeam>,
}

#[derive(Debug, Clone, Default)]
pub struct GreenfieldProgressService;

impl GreenfieldProgressService {
    pub fn new() -> Self {
        Self
    }

    pub fn inventory(&self) -> Vec<GreenfieldSeam> {
        vec![
            seam(
                1,
                "inspect",
                "Self-hosted product-mode summary composition",
                Some("61"),
                GreenfieldSeamStatus::Migrated,
                "Moved product-mode summary composition out of inspect.rs.",
            ),
            seam(
                2,
                "routes",
                "Self-hosted product-mode route family",
                Some("62"),
                GreenfieldSeamStatus::Migrated,
                "Moved the product-mode transition route family behind openrustclaw-app.",
            ),
            seam(
                3,
                "mobile",
                "Mobile node operator report",
                Some("63"),
                GreenfieldSeamStatus::Migrated,
                "Moved mobile operator reporting into the application lane.",
            ),
            seam(
                4,
                "skills",
                "Compiled-skill overview lane",
                Some("64"),
                GreenfieldSeamStatus::Migrated,
                "Moved compiled manifest and artifact overview behavior out of skills.rs.",
            ),
            seam(
                5,
                "inspect",
                "Enterprise admin aggregation",
                Some("65"),
                GreenfieldSeamStatus::Migrated,
                "Moved enterprise admin summary composition out of inspect.rs.",
            ),
            seam(
                6,
                "routes",
                "Enterprise access write route family",
                Some("66"),
                GreenfieldSeamStatus::Migrated,
                "Moved enterprise access write orchestration behind openrustclaw-app.",
            ),
            seam(
                7,
                "skills",
                "Skill install, update, and uninstall mutation lane",
                Some("67"),
                GreenfieldSeamStatus::Migrated,
                "Moved the first mutation-heavy skills lane behind the application seam.",
            ),
            seam(
                8,
                "runtime",
                "Runtime provider/model switch lane",
                Some("68"),
                GreenfieldSeamStatus::Migrated,
                "Moved provider and model switching behind openrustclaw-app.",
            ),
            seam(
                9,
                "skills",
                "Voice-plugin bind lifecycle lane",
                Some("69"),
                GreenfieldSeamStatus::Migrated,
                "Moved voice-plugin binding rules and result shaping out of skills.rs.",
            ),
            seam(
                10,
                "runtime",
                "Runtime vault mutation lane",
                Some("70"),
                GreenfieldSeamStatus::Migrated,
                "Moved runtime vault set/delete mutation behind openrustclaw-app.",
            ),
            seam(
                11,
                "runtime",
                "Runtime reload-planning lane",
                Some("71"),
                GreenfieldSeamStatus::Migrated,
                "Moved reload-plan comparison and classification out of runtime.rs.",
            ),
            seam(
                12,
                "routes",
                "Runtime vault route family",
                Some("72"),
                GreenfieldSeamStatus::Migrated,
                "Moved /control/runtime/vault behind the application seam.",
            ),
            seam(
                13,
                "skills",
                "Auth-plugin lifecycle lane",
                Some("74"),
                GreenfieldSeamStatus::Migrated,
                "Moved auth-plugin binding validation, key derivation, and scope shaping out of skills.rs.",
            ),
            seam(
                14,
                "skills",
                "Channel-extension and background workflow lifecycle lane",
                Some("77"),
                GreenfieldSeamStatus::Migrated,
                "Moved background workflow scheduling and channel-extension binding behind openrustclaw-app.",
            ),
            seam(
                15,
                "runtime",
                "Runtime upgrade planning lane",
                Some("75"),
                GreenfieldSeamStatus::Migrated,
                "Moved upgrade-plan blocker detection and operator-step generation out of runtime.rs.",
            ),
            seam(
                16,
                "runtime",
                "Runtime self-update and rollback planning lane",
                Some("75"),
                GreenfieldSeamStatus::Migrated,
                "Moved self-update and rollback planning guidance out of runtime.rs.",
            ),
            seam(
                17,
                "control",
                "Greenfield progress summary surface",
                Some("76"),
                GreenfieldSeamStatus::Migrated,
                "Added a shipped runtime-maintenance summary surface that reports canonical conversion progress and the remaining queue.",
            ),
            seam(
                18,
                "routes",
                "Runtime maintenance route family",
                Some("76"),
                GreenfieldSeamStatus::Migrated,
                "Added a bounded runtime-maintenance route surface behind openrustclaw-app.",
            ),
        ]
    }

    pub fn report(&self, generated_at: String) -> GreenfieldProgressReport {
        let seams = self.inventory();
        let total_seams = seams.len();
        let completed_seams = seams
            .iter()
            .filter(|seam| seam.status == GreenfieldSeamStatus::Migrated)
            .count();
        let remaining_seams = total_seams.saturating_sub(completed_seams);
        let completion_percent = if total_seams == 0 {
            0
        } else {
            ((completed_seams * 100) / total_seams) as u8
        };

        GreenfieldProgressReport {
            generated_at,
            baseline_name: "Ranked Seam Inventory".to_string(),
            baseline_note: "Counts the ranked follow-on seam inventory after the proving-slice milestone and excludes the initial application-shell bootstrap work.".to_string(),
            ledger_status: if remaining_seams == 0 {
                "complete".to_string()
            } else {
                "active".to_string()
            },
            queue_decision: if remaining_seams == 0 {
                "retire_current_ranked_inventory".to_string()
            } else {
                "continue_current_ranked_inventory".to_string()
            },
            follow_on_note: if remaining_seams == 0 {
                "The current ranked seam inventory is complete at 18/18. Future greenfield follow-on work requires an explicit new canonical inventory instead of silently extending this ledger.".to_string()
            } else {
                "Continue using this ranked seam inventory as the canonical denominator until all remaining seams are migrated or an explicit new inventory supersedes it.".to_string()
            },
            completed_seams,
            remaining_seams,
            total_seams,
            completion_percent,
            seams,
        }
    }
}

fn seam(
    rank: u32,
    area: &str,
    name: &str,
    source_phase: Option<&str>,
    status: GreenfieldSeamStatus,
    rationale: &str,
) -> GreenfieldSeam {
    GreenfieldSeam {
        rank,
        area: area.to_string(),
        name: name.to_string(),
        source_phase: source_phase.map(ToString::to_string),
        status,
        rationale: rationale.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{GreenfieldProgressService, GreenfieldSeamStatus};

    #[test]
    fn report_uses_ranked_inventory_baseline() {
        let report = GreenfieldProgressService::new().report("2026-03-28T16:10:00Z".to_string());

        assert_eq!(report.total_seams, 18);
        assert_eq!(report.completed_seams, 18);
        assert_eq!(report.remaining_seams, 0);
        assert_eq!(report.completion_percent, 100);
        assert_eq!(report.ledger_status, "complete");
        assert_eq!(report.queue_decision, "retire_current_ranked_inventory");
        assert_eq!(report.seams.first().unwrap().rank, 1);
        assert_eq!(report.seams.last().unwrap().rank, 18);
        assert!(
            report
                .seams
                .iter()
                .all(|seam| seam.status == GreenfieldSeamStatus::Migrated)
        );
    }
}
