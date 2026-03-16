//! Memory write policies: deduplication, importance scoring, TTL, confidence.
//!
//! Every memory write goes through policy enforcement before storage.

use openrustclaw_core::types::MemorySource;
use sha2::{Digest, Sha256};

/// Configuration for memory write policies.
#[derive(Debug, Clone)]
pub struct MemoryPolicies {
    pub dedupe_cosine_threshold: f32,
    pub max_core_tokens: usize,
    pub max_core_entries: usize,
    pub embedding_concurrency: usize,
    pub ttl_episodic_days: u64,
    pub ttl_procedural_days: Option<u64>,
    pub ttl_semantic_days: Option<u64>,
    pub consolidation_threshold: usize,
    pub decay_half_life_days: f64,
}

impl Default for MemoryPolicies {
    fn default() -> Self {
        Self {
            dedupe_cosine_threshold: 0.92,
            max_core_tokens: 500,
            max_core_entries: 20,
            embedding_concurrency: 4,
            ttl_episodic_days: 90,
            ttl_procedural_days: None,
            ttl_semantic_days: None,
            consolidation_threshold: 1000,
            decay_half_life_days: 30.0,
        }
    }
}

impl MemoryPolicies {
    /// Compute SHA-256 content hash for deduplication.
    pub fn content_hash(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Score importance based on the source of the memory.
    pub fn score_importance(source: &MemorySource) -> f32 {
        match source {
            MemorySource::ExplicitUserStatement => 1.0,
            MemorySource::UserCorrection => 0.95,
            MemorySource::ToolResult => 0.8,
            MemorySource::AgentInference => 0.7,
            MemorySource::ConversationSummary => 0.6,
            MemorySource::BackgroundIngestion => 0.5,
        }
    }

    /// Apply temporal decay to a base score.
    pub fn decay_score(&self, base_score: f32, days_since_access: f64) -> f32 {
        base_score * (0.5_f32).powf((days_since_access / self.decay_half_life_days) as f32)
    }

    /// Compute cosine similarity between two vectors.
    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if mag_a == 0.0 || mag_b == 0.0 {
            return 0.0;
        }
        dot / (mag_a * mag_b)
    }
}
