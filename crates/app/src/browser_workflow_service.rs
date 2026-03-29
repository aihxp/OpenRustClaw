use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BrowserSessionSummary {
    pub id: String,
    pub label: Option<String>,
    pub backend: String,
    pub created_at: String,
    pub updated_at: String,
    pub state_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BrowserSessionRecord {
    pub id: String,
    pub label: Option<String>,
    pub backend: String,
    pub created_at: String,
    pub updated_at: String,
    pub state_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BrowserWorkflowRecord {
    pub id: String,
    pub action: String,
    pub backend: String,
    pub session_id: Option<String>,
    pub status: String,
    pub final_url: String,
    pub title: String,
    pub step_count: usize,
    pub artifact_path: String,
    pub created_at: String,
    pub result_preview: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BrowserWorkflowHistoryReport {
    pub limit: usize,
    pub entries: Vec<BrowserWorkflowRecord>,
}

#[derive(Debug, Clone, Default)]
pub struct BrowserWorkflowService;

impl BrowserWorkflowService {
    pub fn new() -> Self {
        Self
    }

    pub fn session_record(
        &self,
        session_id: String,
        label: Option<String>,
        backend: String,
        state_path: String,
        created_at: String,
    ) -> BrowserSessionRecord {
        BrowserSessionRecord {
            id: session_id,
            label,
            backend,
            created_at: created_at.clone(),
            updated_at: created_at,
            state_path,
        }
    }

    pub fn session_summary(&self, record: &BrowserSessionRecord) -> BrowserSessionSummary {
        BrowserSessionSummary {
            id: record.id.clone(),
            label: record.label.clone(),
            backend: record.backend.clone(),
            created_at: record.created_at.clone(),
            updated_at: record.updated_at.clone(),
            state_path: record.state_path.clone(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn workflow_record(
        &self,
        action: impl Into<String>,
        backend: impl Into<String>,
        session_id: Option<String>,
        status: impl Into<String>,
        final_url: impl Into<String>,
        title: impl Into<String>,
        step_count: usize,
        artifact_path: impl Into<String>,
        result_preview: Option<Value>,
    ) -> BrowserWorkflowRecord {
        BrowserWorkflowRecord {
            id: Uuid::new_v4().to_string(),
            action: action.into(),
            backend: backend.into(),
            session_id,
            status: status.into(),
            final_url: final_url.into(),
            title: title.into(),
            step_count: step_count.max(1),
            artifact_path: artifact_path.into(),
            created_at: chrono::Utc::now().to_rfc3339(),
            result_preview,
        }
    }

    pub fn workflow_history(
        &self,
        mut entries: Vec<BrowserWorkflowRecord>,
        limit: usize,
        action: Option<&str>,
        backend: Option<&str>,
    ) -> BrowserWorkflowHistoryReport {
        entries.retain(|record| {
            action.is_none_or(|value| record.action == value)
                && backend.is_none_or(|value| record.backend == value)
        });
        entries.sort_by(|left, right| right.created_at.cmp(&left.created_at));
        entries.truncate(limit.max(1));
        BrowserWorkflowHistoryReport {
            limit: limit.max(1),
            entries,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_summary_reflects_record_fields() {
        let service = BrowserWorkflowService::new();
        let record = service.session_record(
            "session-1".to_string(),
            Some("Main".to_string()),
            "native_cdp".to_string(),
            ".claw/browser/state/session-1.json".to_string(),
            "2026-03-28T00:00:00Z".to_string(),
        );
        let summary = service.session_summary(&record);
        assert_eq!(summary.id, "session-1");
        assert_eq!(summary.backend, "native_cdp");
    }

    #[test]
    fn workflow_record_sets_minimum_step_count() {
        let service = BrowserWorkflowService::new();
        let record = service.workflow_record(
            "inspect",
            "native_cdp",
            None,
            "success",
            "https://example.com",
            "Example",
            0,
            ".claw/browser/read/example.json",
            None,
        );
        assert_eq!(record.step_count, 1);
    }

    #[test]
    fn workflow_history_filters_and_sorts_entries() {
        let service = BrowserWorkflowService::new();
        let older = BrowserWorkflowRecord {
            created_at: "2026-03-27T00:00:00Z".to_string(),
            ..service.workflow_record(
                "inspect",
                "native_cdp",
                None,
                "success",
                "https://example.com/docs",
                "Docs",
                1,
                ".claw/browser/read/docs.json",
                None,
            )
        };
        let newer = BrowserWorkflowRecord {
            created_at: "2026-03-28T00:00:00Z".to_string(),
            ..service.workflow_record(
                "run_sequence",
                "native_cdp",
                Some("session-1".to_string()),
                "success",
                "https://example.com/app",
                "App",
                4,
                ".claw/browser/sequences/app.json",
                None,
            )
        };
        let report = service.workflow_history(vec![older, newer], 10, Some("run_sequence"), None);
        assert_eq!(report.entries.len(), 1);
        assert_eq!(report.entries[0].action, "run_sequence");
    }
}
