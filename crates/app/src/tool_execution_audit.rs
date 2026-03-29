use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolExecutionRecord {
    pub id: String,
    pub tool_name: String,
    pub source: String,
    pub status: String,
    pub status_detail: String,
    pub duration_ms: u64,
    pub created_at: String,
    pub error: Option<String>,
    pub artifact_path: Option<String>,
    pub args: Option<Value>,
    pub result_preview: Option<Value>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ToolExecutionHistoryReport {
    pub limit: usize,
    pub entries: Vec<ToolExecutionRecord>,
}

pub struct ToolExecutionAuditService;

impl ToolExecutionAuditService {
    pub fn history(
        records: &[ToolExecutionRecord],
        limit: usize,
        source: Option<&str>,
        status: Option<&str>,
        tool_name: Option<&str>,
    ) -> ToolExecutionHistoryReport {
        let mut entries = records
            .iter()
            .filter(|record| source.is_none_or(|value| record.source == value))
            .filter(|record| status.is_none_or(|value| record.status == value))
            .filter(|record| tool_name.is_none_or(|value| record.tool_name == value))
            .cloned()
            .collect::<Vec<_>>();
        entries.sort_by(|left, right| right.created_at.cmp(&left.created_at));
        entries.truncate(limit.max(1));
        ToolExecutionHistoryReport {
            limit: limit.max(1),
            entries,
        }
    }

    pub fn new_record(
        tool_name: &str,
        source: &str,
        status: &str,
        status_detail: &str,
        duration_ms: u64,
        error: Option<&str>,
        artifact_path: Option<&str>,
        args: Option<Value>,
        result_preview: Option<Value>,
    ) -> ToolExecutionRecord {
        ToolExecutionRecord {
            id: Uuid::new_v4().to_string(),
            tool_name: tool_name.to_string(),
            source: source.to_string(),
            status: status.to_string(),
            status_detail: status_detail.to_string(),
            duration_ms,
            created_at: Utc::now().to_rfc3339(),
            error: error.map(ToString::to_string),
            artifact_path: artifact_path.map(ToString::to_string),
            args,
            result_preview,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ToolExecutionAuditService, ToolExecutionRecord};

    #[test]
    fn history_filters_and_sorts_entries() {
        let records = vec![
            ToolExecutionRecord {
                id: "1".to_string(),
                tool_name: "a".to_string(),
                source: "mcp".to_string(),
                status: "success".to_string(),
                status_detail: "ok".to_string(),
                duration_ms: 1,
                created_at: "2026-03-28T10:00:00Z".to_string(),
                error: None,
                artifact_path: None,
                args: None,
                result_preview: None,
            },
            ToolExecutionRecord {
                id: "2".to_string(),
                tool_name: "b".to_string(),
                source: "cli".to_string(),
                status: "failure".to_string(),
                status_detail: "err".to_string(),
                duration_ms: 2,
                created_at: "2026-03-28T11:00:00Z".to_string(),
                error: Some("boom".to_string()),
                artifact_path: None,
                args: None,
                result_preview: None,
            },
        ];

        let report = ToolExecutionAuditService::history(&records, 5, Some("cli"), None, None);
        assert_eq!(report.entries.len(), 1);
        assert_eq!(report.entries[0].id, "2");
    }
}
