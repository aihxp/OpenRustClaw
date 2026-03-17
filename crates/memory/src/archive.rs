//! Archive memory: compressed long-term storage.
//!
//! Old, low-value memories are consolidated into summaries here.
//! Only searched when recall memory returns insufficient results.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// An archived memory summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveEntry {
    pub id: String,
    pub summary: String,
    pub source_memory_ids: Vec<String>,
    pub source_type: Option<String>,
    pub namespace: String,
    pub importance: f32,
    pub created_at: DateTime<Utc>,
}

/// Criteria for selecting memories to archive.
#[derive(Debug, Clone)]
pub struct ArchiveCriteria {
    /// Memories older than this many days are candidates.
    pub min_age_days: u64,
    /// Memories accessed fewer than this many times are candidates.
    pub max_access_count: u32,
    /// Minimum number of entries before triggering archival.
    pub min_entries_threshold: usize,
}

impl Default for ArchiveCriteria {
    fn default() -> Self {
        Self {
            min_age_days: 60,
            max_access_count: 2,
            min_entries_threshold: 1000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archive_criteria_defaults() {
        let c = ArchiveCriteria::default();
        assert_eq!(c.min_age_days, 60);
        assert_eq!(c.max_access_count, 2);
        assert_eq!(c.min_entries_threshold, 1000);
    }

    #[test]
    fn archive_entry_serialization_roundtrip() {
        let entry = ArchiveEntry {
            id: "archive-001".to_string(),
            summary: "User prefers dark mode and Rust programming.".to_string(),
            source_memory_ids: vec![
                "mem-1".to_string(),
                "mem-2".to_string(),
                "mem-3".to_string(),
            ],
            source_type: Some("conversation".to_string()),
            namespace: "default".to_string(),
            importance: 0.85,
            created_at: Utc::now(),
        };

        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: ArchiveEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, "archive-001");
        assert_eq!(
            deserialized.summary,
            "User prefers dark mode and Rust programming."
        );
        assert_eq!(deserialized.source_memory_ids.len(), 3);
        assert_eq!(deserialized.source_type, Some("conversation".to_string()));
        assert_eq!(deserialized.namespace, "default");
        assert!((deserialized.importance - 0.85).abs() < 1e-6);
    }

    #[test]
    fn archive_entry_no_source_type() {
        let entry = ArchiveEntry {
            id: "archive-002".to_string(),
            summary: "General knowledge.".to_string(),
            source_memory_ids: vec![],
            source_type: None,
            namespace: "global".to_string(),
            importance: 0.5,
            created_at: Utc::now(),
        };

        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: ArchiveEntry = serde_json::from_str(&json).unwrap();
        assert!(deserialized.source_type.is_none());
        assert!(deserialized.source_memory_ids.is_empty());
    }

    #[test]
    fn archive_criteria_custom_values() {
        let c = ArchiveCriteria {
            min_age_days: 30,
            max_access_count: 5,
            min_entries_threshold: 500,
        };
        assert_eq!(c.min_age_days, 30);
        assert_eq!(c.max_access_count, 5);
        assert_eq!(c.min_entries_threshold, 500);
    }
}
