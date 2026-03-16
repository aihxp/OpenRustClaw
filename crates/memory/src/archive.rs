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
