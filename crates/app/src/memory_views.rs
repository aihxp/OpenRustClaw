use chrono::Utc;
use openrustclaw_core::types::{RetrievalArtifactKind, RetrievalVectorLane};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CoreMemoryItem {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecallMemoryItem {
    pub id: String,
    pub memory_type: String,
    pub namespace: String,
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub importance: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_accessed: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub age_seconds: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lexical_score: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vector_score: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recency_score: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence_score: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub importance_score: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fused_score: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vector_lane: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub degraded_reason: Option<String>,
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
            if let Some(score) = entry.score {
                out.push_str(&format!("- score: {:.2}\n", score));
            }
            if let Some(importance) = entry.importance {
                out.push_str(&format!("- importance: {:.2}\n", importance));
            }
            if let Some(confidence) = entry.confidence {
                out.push_str(&format!("- confidence: {:.2}\n", confidence));
            }
            if let Some(artifact_kind) = &entry.artifact_kind {
                out.push_str(&format!("- artifact_kind: {}\n", artifact_kind));
            }
            if let Some(source_type) = &entry.source_type {
                out.push_str(&format!("- source_type: {}\n", source_type));
            }
            if let Some(source_label) = &entry.source_label {
                out.push_str(&format!("- source_label: {}\n", source_label));
            }
            if let Some(created_at) = &entry.created_at {
                out.push_str(&format!("- created_at: {}\n", created_at));
            }
            if let Some(last_accessed) = &entry.last_accessed {
                out.push_str(&format!("- last_accessed: {}\n", last_accessed));
            }
            if let Some(age_seconds) = entry.age_seconds {
                out.push_str(&format!("- age_seconds: {}\n", age_seconds));
            }
            if let Some(lexical_score) = entry.lexical_score {
                out.push_str(&format!("- lexical_score: {:.2}\n", lexical_score));
            }
            if let Some(vector_score) = entry.vector_score {
                out.push_str(&format!("- vector_score: {:.2}\n", vector_score));
            }
            if let Some(recency_score) = entry.recency_score {
                out.push_str(&format!("- recency_score: {:.2}\n", recency_score));
            }
            if let Some(confidence_score) = entry.confidence_score {
                out.push_str(&format!("- confidence_score: {:.2}\n", confidence_score));
            }
            if let Some(importance_score) = entry.importance_score {
                out.push_str(&format!("- importance_score: {:.2}\n", importance_score));
            }
            if let Some(fused_score) = entry.fused_score {
                out.push_str(&format!("- fused_score: {:.2}\n", fused_score));
            }
            if let Some(vector_lane) = &entry.vector_lane {
                out.push_str(&format!("- vector_lane: {}\n", vector_lane));
            }
            if let Some(degraded_reason) = &entry.degraded_reason {
                out.push_str(&format!("- degraded_reason: {}\n", degraded_reason));
            }
            out.push('\n');
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
        let mut current_header: Option<RecallMemoryItem> = None;
        let mut current_body = String::new();
        for line in content.lines() {
            if let Some(rest) = line.strip_prefix("## ") {
                if let Some(mut entry) = current_header.take() {
                    entry.content = current_body.trim().to_string();
                    entries.push(entry);
                }
                let parts: Vec<_> = rest
                    .split('|')
                    .map(|part| part.trim().to_string())
                    .collect();
                if parts.len() == 3 {
                    current_header = Some(RecallMemoryItem {
                        id: parts[0].clone(),
                        memory_type: parts[1].clone(),
                        namespace: parts[2].clone(),
                        content: String::new(),
                        score: None,
                        importance: None,
                        confidence: None,
                        artifact_kind: None,
                        source_type: None,
                        source_label: None,
                        created_at: None,
                        last_accessed: None,
                        age_seconds: None,
                        lexical_score: None,
                        vector_score: None,
                        recency_score: None,
                        confidence_score: None,
                        importance_score: None,
                        fused_score: None,
                        vector_lane: None,
                        degraded_reason: None,
                    });
                }
                current_body.clear();
                continue;
            }
            if let Some(rest) = line.strip_prefix("- ") {
                if let Some(entry) = current_header.as_mut() {
                    if let Some((key, value)) = rest.split_once(':') {
                        let key = key.trim();
                        let value = value.trim();
                        match key {
                            "score" => entry.score = value.parse().ok(),
                            "importance" => entry.importance = value.parse().ok(),
                            "confidence" => entry.confidence = value.parse().ok(),
                            "artifact_kind" => entry.artifact_kind = Some(value.to_string()),
                            "source_type" => entry.source_type = Some(value.to_string()),
                            "source_label" => entry.source_label = Some(value.to_string()),
                            "created_at" => entry.created_at = Some(value.to_string()),
                            "last_accessed" => entry.last_accessed = Some(value.to_string()),
                            "age_seconds" => entry.age_seconds = value.parse().ok(),
                            "lexical_score" => entry.lexical_score = value.parse().ok(),
                            "vector_score" => entry.vector_score = value.parse().ok(),
                            "recency_score" => entry.recency_score = value.parse().ok(),
                            "confidence_score" => entry.confidence_score = value.parse().ok(),
                            "importance_score" => entry.importance_score = value.parse().ok(),
                            "fused_score" => entry.fused_score = value.parse().ok(),
                            "vector_lane" => entry.vector_lane = Some(value.to_string()),
                            "degraded_reason" => entry.degraded_reason = Some(value.to_string()),
                            _ => {}
                        }
                    }
                }
                continue;
            }
            if current_header.is_some() {
                current_body.push_str(line);
                current_body.push('\n');
            }
        }
        if let Some(mut entry) = current_header.take() {
            entry.content = current_body.trim().to_string();
            entries.push(entry);
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

    pub fn artifact_kind_label(&self, value: RetrievalArtifactKind) -> &'static str {
        match value {
            RetrievalArtifactKind::RecallMemory => "recall_memory",
            RetrievalArtifactKind::ConversationMemory => "conversation_memory",
            RetrievalArtifactKind::DocumentChunk => "document_chunk",
            RetrievalArtifactKind::CodeChunk => "code_chunk",
            RetrievalArtifactKind::ConfigFragment => "config_fragment",
            RetrievalArtifactKind::RunbookStep => "runbook_step",
            RetrievalArtifactKind::ToolSchema => "tool_schema",
            RetrievalArtifactKind::ArchiveSummary => "archive_summary",
        }
    }

    pub fn vector_lane_label(&self, value: RetrievalVectorLane) -> &'static str {
        match value {
            RetrievalVectorLane::NativeLibsql => "native_libsql",
            RetrievalVectorLane::RustRescored => "rust_rescored",
            RetrievalVectorLane::Unavailable => "unavailable",
        }
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

    #[test]
    fn recall_view_round_trips_explanation_metadata() {
        let service = MemoryViewsService::new();
        let rendered = service.render_recall_view(
            "user-1",
            &[RecallMemoryItem {
                id: "abc".to_string(),
                memory_type: "semantic".to_string(),
                namespace: "user-1".to_string(),
                content: "Remember the operator preference".to_string(),
                score: Some(0.92),
                importance: Some(0.80),
                confidence: Some(0.90),
                artifact_kind: Some("conversation_memory".to_string()),
                source_type: Some("conversation".to_string()),
                source_label: Some("test-source".to_string()),
                created_at: Some("2026-04-08T00:00:00Z".to_string()),
                last_accessed: None,
                age_seconds: Some(60),
                lexical_score: Some(0.85),
                vector_score: Some(0.75),
                recency_score: Some(0.90),
                confidence_score: Some(0.90),
                importance_score: Some(0.80),
                fused_score: Some(0.92),
                vector_lane: Some("rust_rescored".to_string()),
                degraded_reason: Some("query embeddings unavailable".to_string()),
            }],
        );

        let parsed = service.parse_recall_view(&rendered);
        assert_eq!(parsed.len(), 1);
        assert_eq!(
            parsed[0].artifact_kind.as_deref(),
            Some("conversation_memory")
        );
        assert_eq!(parsed[0].vector_lane.as_deref(), Some("rust_rescored"));
        assert_eq!(
            parsed[0].degraded_reason.as_deref(),
            Some("query embeddings unavailable")
        );
    }
}
