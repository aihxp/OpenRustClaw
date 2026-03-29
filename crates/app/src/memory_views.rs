use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CoreMemoryItem {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecallMemoryItem {
    pub id: String,
    pub memory_type: String,
    pub namespace: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArchiveMemoryItem {
    pub id: String,
    pub summary: String,
}

#[derive(Debug, Clone, Default)]
pub struct MemoryViewsService;

impl MemoryViewsService {
    pub fn new() -> Self {
        Self
    }

    pub fn render_core_view(&self, user_id: &str, entries: &[CoreMemoryItem]) -> String {
        let mut out = format!("# Core Memory\n\nUser: {}\n\n", user_id);
        for entry in entries {
            out.push_str(&format!("## {}\n{}\n\n", entry.key, entry.value));
        }
        out
    }

    pub fn render_recall_view(&self, user_id: &str, entries: &[RecallMemoryItem]) -> String {
        let mut out = format!("# Recall Memory\n\nUser: {}\n\n", user_id);
        for entry in entries {
            out.push_str(&format!(
                "## {} | {} | {}\n{}\n\n",
                entry.id, entry.memory_type, entry.namespace, entry.content
            ));
        }
        out
    }

    pub fn render_archive_view(&self, namespace: &str, entries: &[ArchiveMemoryItem]) -> String {
        let mut out = format!("# Archive Memory\n\nNamespace: {}\n\n", namespace);
        for entry in entries {
            out.push_str(&format!("## {}\n{}\n\n", entry.id, entry.summary));
        }
        out
    }

    pub fn parse_core_view(&self, content: &str) -> Vec<CoreMemoryItem> {
        let mut entries = Vec::new();
        let mut current_key = None;
        let mut current_value = String::new();
        for line in content.lines() {
            if let Some(rest) = line.strip_prefix("## ") {
                if let Some(key) = current_key.take() {
                    entries.push(CoreMemoryItem {
                        key,
                        value: current_value.trim().to_string(),
                    });
                }
                current_key = Some(rest.trim().to_string());
                current_value.clear();
                continue;
            }
            if current_key.is_some() {
                current_value.push_str(line);
                current_value.push('\n');
            }
        }
        if let Some(key) = current_key.take() {
            entries.push(CoreMemoryItem {
                key,
                value: current_value.trim().to_string(),
            });
        }
        entries
    }

    pub fn parse_recall_view(&self, content: &str) -> Vec<RecallMemoryItem> {
        let mut entries = Vec::new();
        let mut current_header: Option<(String, String, String)> = None;
        let mut current_body = String::new();
        for line in content.lines() {
            if let Some(rest) = line.strip_prefix("## ") {
                if let Some((id, memory_type, namespace)) = current_header.take() {
                    entries.push(RecallMemoryItem {
                        id,
                        memory_type,
                        namespace,
                        content: current_body.trim().to_string(),
                    });
                }
                let parts: Vec<_> = rest
                    .split('|')
                    .map(|part| part.trim().to_string())
                    .collect();
                if parts.len() == 3 {
                    current_header = Some((parts[0].clone(), parts[1].clone(), parts[2].clone()));
                }
                current_body.clear();
                continue;
            }
            if current_header.is_some() {
                current_body.push_str(line);
                current_body.push('\n');
            }
        }
        if let Some((id, memory_type, namespace)) = current_header.take() {
            entries.push(RecallMemoryItem {
                id,
                memory_type,
                namespace,
                content: current_body.trim().to_string(),
            });
        }
        entries
    }

    pub fn assistant_write_policy_summary(&self, metadata: &Value) -> Option<String> {
        let policy = metadata.get("assistant_write_policy")?;
        let basis = policy.get("basis")?.as_str()?;
        let reason = policy
            .get("declared_reason")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        if reason.is_empty() {
            Some(format!("basis={basis}"))
        } else {
            Some(format!("basis={basis}; reason={reason}"))
        }
    }

    pub fn parse_memory_type(&self, value: &str) -> Option<&'static str> {
        match value.to_lowercase().as_str() {
            "episodic" => Some("episodic"),
            "semantic" => Some("semantic"),
            "procedural" => Some("procedural"),
            _ => None,
        }
    }

    pub fn parse_source_type(&self, value: &str) -> Option<&'static str> {
        match value.to_lowercase().as_str() {
            "document" => Some("document"),
            "code" => Some("code"),
            "config" => Some("config"),
            "conversation" => Some("conversation"),
            "runbook" => Some("runbook"),
            "tool_schema" | "tool-schema" => Some("tool_schema"),
            _ => None,
        }
    }

    pub fn import_timestamp(&self) -> String {
        Utc::now().to_rfc3339()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_render_recall_views_round_trip_headers() {
        let service = MemoryViewsService::new();
        let content = "# Recall Memory\n\nUser: user-1\n\n## abc | semantic | user-1\nhello\n\n";
        let entries = service.parse_recall_view(content);
        assert_eq!(entries[0].memory_type, "semantic");
        let rendered = service.render_recall_view("user-1", &entries);
        assert!(rendered.contains("## abc | semantic | user-1"));
    }

    #[test]
    fn assistant_write_policy_summary_formats_reason() {
        let summary =
            MemoryViewsService::new().assistant_write_policy_summary(&serde_json::json!({
                "assistant_write_policy": {
                    "basis": "operator_approved",
                    "declared_reason": "important"
                }
            }));
        assert_eq!(
            summary.as_deref(),
            Some("basis=operator_approved; reason=important")
        );
    }
}
