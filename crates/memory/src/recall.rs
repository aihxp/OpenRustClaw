//! Recall memory: searchable database of everything the agent knows.
//!
//! NEVER injected into the system prompt. Agent accesses this via the
//! `memory_search` tool on demand.

use crate::policies::MemoryPolicies;
use chrono::Utc;
use openrustclaw_core::types::{MemoryEntry, MemorySource, MemoryType, ScoredMemory};
use tracing::debug;
use uuid::Uuid;

/// Recall memory manager.
pub struct RecallMemory {
    policies: MemoryPolicies,
}

impl RecallMemory {
    pub fn new(policies: MemoryPolicies) -> Self {
        Self { policies }
    }

    /// Prepare a new memory entry with policy enforcement.
    /// Returns the entry ready for storage, or an error if it's a duplicate.
    pub fn prepare_entry(
        &self,
        content: &str,
        memory_type: MemoryType,
        source: MemorySource,
        user_id: Option<&str>,
        session_id: Option<Uuid>,
        namespace: Option<&str>,
    ) -> MemoryEntry {
        let content_hash = MemoryPolicies::content_hash(content);
        let importance = MemoryPolicies::score_importance(&source);

        let expires_at = match memory_type {
            MemoryType::Episodic if self.policies.ttl_episodic_days > 0 => Some(
                Utc::now()
                    + chrono::Duration::days(self.policies.ttl_episodic_days as i64),
            ),
            _ => None,
        };

        let entry = MemoryEntry {
            id: Uuid::new_v4(),
            memory_type,
            content: content.to_string(),
            content_hash,
            source: Some(format!("{}", source)),
            source_type: None,
            session_id,
            user_id: user_id.map(String::from),
            namespace: namespace.unwrap_or("global").to_string(),
            importance,
            confidence: 1.0,
            access_count: 0,
            last_accessed: None,
            created_at: Utc::now(),
            expires_at,
            metadata: serde_json::Value::Object(serde_json::Map::new()),
        };

        debug!(id = %entry.id, importance = importance, "Prepared memory entry");
        entry
    }

    /// Apply temporal decay to scored results.
    pub fn apply_decay(&self, results: &mut Vec<ScoredMemory>) {
        let now = Utc::now();
        for result in results.iter_mut() {
            if let Some(last_accessed) = result.entry.last_accessed {
                let days = (now - last_accessed).num_days() as f64;
                result.score = self.policies.decay_score(result.score, days);
            }
        }
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    /// Get the policies.
    pub fn policies(&self) -> &MemoryPolicies {
        &self.policies
    }
}
