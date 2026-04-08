//! Recall memory: searchable database of everything the agent knows.
//!
//! NEVER injected into the system prompt. Agent accesses this via the
//! `memory_search` tool on demand.

use crate::policies::MemoryPolicies;
use chrono::Utc;
use openrustclaw_core::types::{
    MemoryEntry, MemorySource, MemoryType, RetrievalExplanation, ScoredMemory,
};
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
            MemoryType::Episodic if self.policies.ttl_episodic_days > 0 => {
                Some(Utc::now() + chrono::Duration::days(self.policies.ttl_episodic_days as i64))
            }
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
    pub fn apply_decay(&self, results: &mut [ScoredMemory]) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::DateTime;

    fn default_recall() -> RecallMemory {
        RecallMemory::new(MemoryPolicies::default())
    }

    fn make_scored_memory(
        content: &str,
        score: f32,
        last_accessed: Option<DateTime<Utc>>,
    ) -> ScoredMemory {
        ScoredMemory {
            entry: MemoryEntry {
                id: Uuid::new_v4(),
                memory_type: MemoryType::Semantic,
                content: content.to_string(),
                content_hash: MemoryPolicies::content_hash(content),
                source: None,
                source_type: None,
                session_id: None,
                user_id: None,
                namespace: "global".to_string(),
                importance: 0.8,
                confidence: 1.0,
                access_count: 0,
                last_accessed,
                created_at: Utc::now(),
                expires_at: None,
                metadata: serde_json::Value::Object(serde_json::Map::new()),
            },
            score,
            explanation: RetrievalExplanation::default(),
        }
    }

    // ── prepare_entry tests ──

    #[test]
    fn prepare_entry_sets_content_hash() {
        let recall = default_recall();
        let entry = recall.prepare_entry(
            "test content",
            MemoryType::Semantic,
            MemorySource::ExplicitUserStatement,
            None,
            None,
            None,
        );
        assert_eq!(
            entry.content_hash,
            MemoryPolicies::content_hash("test content")
        );
    }

    #[test]
    fn prepare_entry_sets_importance_from_source() {
        let recall = default_recall();
        let entry = recall.prepare_entry(
            "test",
            MemoryType::Semantic,
            MemorySource::ToolResult,
            None,
            None,
            None,
        );
        assert_eq!(entry.importance, 0.8);
    }

    #[test]
    fn prepare_entry_episodic_gets_expiry() {
        let recall = default_recall();
        let entry = recall.prepare_entry(
            "something happened",
            MemoryType::Episodic,
            MemorySource::AgentInference,
            None,
            None,
            None,
        );
        assert!(
            entry.expires_at.is_some(),
            "Episodic memories should have an expiry"
        );
        let expires = entry.expires_at.unwrap();
        let days_until = (expires - Utc::now()).num_days();
        // Should be roughly 90 days from now (default ttl_episodic_days)
        assert!(
            days_until >= 89 && days_until <= 91,
            "Expected ~90 days, got {}",
            days_until
        );
    }

    #[test]
    fn prepare_entry_semantic_no_expiry() {
        let recall = default_recall();
        let entry = recall.prepare_entry(
            "a fact",
            MemoryType::Semantic,
            MemorySource::ExplicitUserStatement,
            None,
            None,
            None,
        );
        assert!(
            entry.expires_at.is_none(),
            "Semantic memories should not expire by default"
        );
    }

    #[test]
    fn prepare_entry_procedural_no_expiry() {
        let recall = default_recall();
        let entry = recall.prepare_entry(
            "how to do X",
            MemoryType::Procedural,
            MemorySource::ToolResult,
            None,
            None,
            None,
        );
        assert!(
            entry.expires_at.is_none(),
            "Procedural memories should not expire by default"
        );
    }

    #[test]
    fn prepare_entry_default_namespace() {
        let recall = default_recall();
        let entry = recall.prepare_entry(
            "test",
            MemoryType::Semantic,
            MemorySource::AgentInference,
            None,
            None,
            None,
        );
        assert_eq!(entry.namespace, "global");
    }

    #[test]
    fn prepare_entry_custom_namespace() {
        let recall = default_recall();
        let entry = recall.prepare_entry(
            "test",
            MemoryType::Semantic,
            MemorySource::AgentInference,
            None,
            None,
            Some("project-alpha"),
        );
        assert_eq!(entry.namespace, "project-alpha");
    }

    #[test]
    fn prepare_entry_with_user_and_session() {
        let recall = default_recall();
        let session = Uuid::new_v4();
        let entry = recall.prepare_entry(
            "user-specific",
            MemoryType::Episodic,
            MemorySource::ExplicitUserStatement,
            Some("user_42"),
            Some(session),
            None,
        );
        assert_eq!(entry.user_id.as_deref(), Some("user_42"));
        assert_eq!(entry.session_id, Some(session));
    }

    #[test]
    fn prepare_entry_initial_confidence_is_one() {
        let recall = default_recall();
        let entry = recall.prepare_entry(
            "confident",
            MemoryType::Semantic,
            MemorySource::ExplicitUserStatement,
            None,
            None,
            None,
        );
        assert_eq!(entry.confidence, 1.0);
    }

    #[test]
    fn prepare_entry_initial_access_count_is_zero() {
        let recall = default_recall();
        let entry = recall.prepare_entry(
            "fresh",
            MemoryType::Semantic,
            MemorySource::AgentInference,
            None,
            None,
            None,
        );
        assert_eq!(entry.access_count, 0);
    }

    // ── apply_decay tests ──

    #[test]
    fn apply_decay_no_last_accessed_keeps_score() {
        let recall = default_recall();
        let mut results = vec![make_scored_memory("test", 0.9, None)];
        recall.apply_decay(&mut results);
        assert!(
            (results[0].score - 0.9).abs() < 1e-6,
            "No last_accessed = no decay"
        );
    }

    #[test]
    fn apply_decay_recent_access_minimal_decay() {
        let recall = default_recall();
        let recent = Utc::now() - chrono::Duration::hours(1);
        let mut results = vec![make_scored_memory("test", 1.0, Some(recent))];
        recall.apply_decay(&mut results);
        // Less than 1 day old, so minimal decay
        assert!(
            results[0].score > 0.95,
            "Recent access should have minimal decay, got {}",
            results[0].score
        );
    }

    #[test]
    fn apply_decay_sorts_by_score_descending() {
        let recall = default_recall();
        let old = Utc::now() - chrono::Duration::days(60);
        let recent = Utc::now() - chrono::Duration::days(1);
        let mut results = vec![
            make_scored_memory("old", 0.8, Some(old)),
            make_scored_memory("recent", 0.8, Some(recent)),
        ];
        recall.apply_decay(&mut results);
        assert!(
            results[0].score >= results[1].score,
            "Results should be sorted descending by score after decay"
        );
        // The recent one should have higher score
        assert!(results[0].entry.content == "recent");
    }

    #[test]
    fn apply_decay_empty_results_no_panic() {
        let recall = default_recall();
        let mut results: Vec<ScoredMemory> = vec![];
        recall.apply_decay(&mut results);
        assert!(results.is_empty());
    }

    // ── policies accessor ──

    #[test]
    fn policies_accessor_returns_configured_policies() {
        let policies = MemoryPolicies {
            max_core_tokens: 999,
            ..Default::default()
        };
        let recall = RecallMemory::new(policies);
        assert_eq!(recall.policies().max_core_tokens, 999);
    }
}
