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

    /// Check whether a candidate embedding is a duplicate of an existing one.
    pub fn is_duplicate(&self, candidate: &[f32], existing: &[f32]) -> bool {
        Self::cosine_similarity(candidate, existing) >= self.dedupe_cosine_threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Content hash tests ──

    #[test]
    fn content_hash_deterministic_same_input() {
        let h1 = MemoryPolicies::content_hash("the quick brown fox");
        let h2 = MemoryPolicies::content_hash("the quick brown fox");
        assert_eq!(h1, h2, "Same input must produce identical hashes");
    }

    #[test]
    fn content_hash_different_inputs_differ() {
        let h1 = MemoryPolicies::content_hash("hello world");
        let h2 = MemoryPolicies::content_hash("hello World");
        assert_ne!(h1, h2, "Different inputs must produce different hashes");
    }

    #[test]
    fn content_hash_is_hex_sha256_length() {
        let hash = MemoryPolicies::content_hash("test");
        // SHA-256 produces 64 hex chars
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn content_hash_empty_string() {
        let hash = MemoryPolicies::content_hash("");
        assert_eq!(
            hash.len(),
            64,
            "Empty string still produces a valid SHA-256 hash"
        );
        // Known SHA-256 of empty string
        assert_eq!(
            hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn content_hash_whitespace_sensitivity() {
        let h1 = MemoryPolicies::content_hash("foo bar");
        let h2 = MemoryPolicies::content_hash("foo  bar");
        assert_ne!(
            h1, h2,
            "Whitespace differences must produce different hashes"
        );
    }

    // ── Importance scoring tests ──

    #[test]
    fn importance_explicit_user_statement_is_highest() {
        let score = MemoryPolicies::score_importance(&MemorySource::ExplicitUserStatement);
        assert_eq!(score, 1.0);
    }

    #[test]
    fn importance_user_correction_is_very_high() {
        let score = MemoryPolicies::score_importance(&MemorySource::UserCorrection);
        assert_eq!(score, 0.95);
    }

    #[test]
    fn importance_tool_result() {
        let score = MemoryPolicies::score_importance(&MemorySource::ToolResult);
        assert_eq!(score, 0.8);
    }

    #[test]
    fn importance_agent_inference() {
        let score = MemoryPolicies::score_importance(&MemorySource::AgentInference);
        assert_eq!(score, 0.7);
    }

    #[test]
    fn importance_conversation_summary() {
        let score = MemoryPolicies::score_importance(&MemorySource::ConversationSummary);
        assert_eq!(score, 0.6);
    }

    #[test]
    fn importance_background_ingestion_is_lowest() {
        let score = MemoryPolicies::score_importance(&MemorySource::BackgroundIngestion);
        assert_eq!(score, 0.5);
    }

    #[test]
    fn importance_ordering_preserved() {
        let sources = [
            MemorySource::ExplicitUserStatement,
            MemorySource::UserCorrection,
            MemorySource::ToolResult,
            MemorySource::AgentInference,
            MemorySource::ConversationSummary,
            MemorySource::BackgroundIngestion,
        ];
        let scores: Vec<f32> = sources
            .iter()
            .map(MemoryPolicies::score_importance)
            .collect();
        for i in 1..scores.len() {
            assert!(
                scores[i - 1] > scores[i],
                "Scores must be strictly decreasing: {:?}",
                scores
            );
        }
    }

    // ── Decay score tests ──

    #[test]
    fn decay_zero_days_returns_base_score() {
        let policies = MemoryPolicies::default();
        let result = policies.decay_score(1.0, 0.0);
        assert!((result - 1.0).abs() < 1e-6, "No decay at day 0");
    }

    #[test]
    fn decay_at_half_life_returns_half() {
        let policies = MemoryPolicies::default();
        // default half life is 30 days
        let result = policies.decay_score(1.0, 30.0);
        assert!(
            (result - 0.5).abs() < 0.01,
            "Score should be ~0.5 at half-life, got {}",
            result
        );
    }

    #[test]
    fn decay_at_two_half_lives_returns_quarter() {
        let policies = MemoryPolicies::default();
        let result = policies.decay_score(1.0, 60.0);
        assert!(
            (result - 0.25).abs() < 0.01,
            "Score should be ~0.25 at two half-lives, got {}",
            result
        );
    }

    #[test]
    fn decay_scales_with_base_score() {
        let policies = MemoryPolicies::default();
        let base = 0.8;
        let result = policies.decay_score(base, 30.0);
        assert!(
            (result - 0.4).abs() < 0.01,
            "0.8 * 0.5 should be ~0.4, got {}",
            result
        );
    }

    #[test]
    fn decay_custom_half_life() {
        let policies = MemoryPolicies {
            decay_half_life_days: 10.0,
            ..Default::default()
        };
        let result = policies.decay_score(1.0, 10.0);
        assert!(
            (result - 0.5).abs() < 0.01,
            "Custom half-life of 10 days: score at 10 days should be ~0.5, got {}",
            result
        );
    }

    // ── Cosine similarity tests ──

    #[test]
    fn cosine_identical_vectors() {
        let v = vec![1.0, 2.0, 3.0];
        let sim = MemoryPolicies::cosine_similarity(&v, &v);
        assert!(
            (sim - 1.0).abs() < 1e-6,
            "Self-similarity should be 1.0, got {}",
            sim
        );
    }

    #[test]
    fn cosine_opposite_vectors() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![-1.0, 0.0, 0.0];
        let sim = MemoryPolicies::cosine_similarity(&a, &b);
        assert!(
            (sim - (-1.0)).abs() < 1e-6,
            "Opposite vectors should have similarity -1.0, got {}",
            sim
        );
    }

    #[test]
    fn cosine_orthogonal_vectors() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let sim = MemoryPolicies::cosine_similarity(&a, &b);
        assert!(
            sim.abs() < 1e-6,
            "Orthogonal vectors should have similarity 0.0, got {}",
            sim
        );
    }

    #[test]
    fn cosine_symmetry() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![4.0, 5.0, 6.0];
        let sim_ab = MemoryPolicies::cosine_similarity(&a, &b);
        let sim_ba = MemoryPolicies::cosine_similarity(&b, &a);
        assert!(
            (sim_ab - sim_ba).abs() < 1e-6,
            "Cosine similarity must be symmetric"
        );
    }

    #[test]
    fn cosine_bounded() {
        let a = vec![3.0, -1.0, 2.0, 5.0];
        let b = vec![-2.0, 4.0, 1.0, -3.0];
        let sim = MemoryPolicies::cosine_similarity(&a, &b);
        assert!(
            sim >= -1.0 && sim <= 1.0,
            "Cosine similarity must be in [-1, 1], got {}",
            sim
        );
    }

    #[test]
    fn cosine_zero_vector_returns_zero() {
        let a = vec![0.0, 0.0, 0.0];
        let b = vec![1.0, 2.0, 3.0];
        assert_eq!(MemoryPolicies::cosine_similarity(&a, &b), 0.0);
        assert_eq!(MemoryPolicies::cosine_similarity(&b, &a), 0.0);
    }

    #[test]
    fn cosine_both_zero_vectors() {
        let a = vec![0.0, 0.0];
        let b = vec![0.0, 0.0];
        assert_eq!(MemoryPolicies::cosine_similarity(&a, &b), 0.0);
    }

    #[test]
    fn cosine_mismatched_lengths_returns_zero() {
        let a = vec![1.0, 2.0];
        let b = vec![1.0, 2.0, 3.0];
        assert_eq!(MemoryPolicies::cosine_similarity(&a, &b), 0.0);
    }

    #[test]
    fn cosine_empty_vectors_returns_zero() {
        let a: Vec<f32> = vec![];
        let b: Vec<f32> = vec![];
        assert_eq!(MemoryPolicies::cosine_similarity(&a, &b), 0.0);
    }

    #[test]
    fn cosine_parallel_scaled_vectors() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![2.0, 4.0, 6.0]; // 2x of a
        let sim = MemoryPolicies::cosine_similarity(&a, &b);
        assert!(
            (sim - 1.0).abs() < 1e-6,
            "Parallel vectors (scaled) should have similarity 1.0, got {}",
            sim
        );
    }

    // ── Dedup threshold tests ──

    #[test]
    fn is_duplicate_above_threshold() {
        let policies = MemoryPolicies {
            dedupe_cosine_threshold: 0.9,
            ..Default::default()
        };
        // Identical vectors have similarity 1.0 >= 0.9
        let v = vec![1.0, 2.0, 3.0];
        assert!(policies.is_duplicate(&v, &v));
    }

    #[test]
    fn is_duplicate_below_threshold() {
        let policies = MemoryPolicies {
            dedupe_cosine_threshold: 0.9,
            ..Default::default()
        };
        // Orthogonal vectors have similarity 0.0 < 0.9
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        assert!(!policies.is_duplicate(&a, &b));
    }

    #[test]
    fn is_duplicate_exactly_at_threshold() {
        let policies = MemoryPolicies {
            dedupe_cosine_threshold: 0.95,
            ..Default::default()
        };
        // Two nearly identical vectors with similarity just above 0.95
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.05, 0.0]; // very close to a
        let sim = MemoryPolicies::cosine_similarity(&a, &b);
        assert!(
            sim > 0.95,
            "These vectors should have similarity > 0.95, got {}",
            sim
        );
        assert!(policies.is_duplicate(&a, &b));
    }

    // ── Default policy values ──

    #[test]
    fn default_policies_have_expected_values() {
        let p = MemoryPolicies::default();
        assert_eq!(p.dedupe_cosine_threshold, 0.92);
        assert_eq!(p.max_core_tokens, 500);
        assert_eq!(p.max_core_entries, 20);
        assert_eq!(p.embedding_concurrency, 4);
        assert_eq!(p.ttl_episodic_days, 90);
        assert_eq!(p.ttl_procedural_days, None);
        assert_eq!(p.ttl_semantic_days, None);
        assert_eq!(p.consolidation_threshold, 1000);
        assert!((p.decay_half_life_days - 30.0).abs() < 1e-6);
    }

    // ── Property-based tests ──

    mod proptest_policies {
        use super::*;
        use proptest::prelude::*;

        /// Generate a non-zero-length vector of finite f32 values.
        fn finite_vec(len: usize) -> impl Strategy<Value = Vec<f32>> {
            prop::collection::vec(-1e6_f32..1e6_f32, len..=len)
        }

        proptest! {
            #[test]
            fn cosine_similarity_is_symmetric(
                a in finite_vec(8),
                b in finite_vec(8),
            ) {
                let sim_ab = MemoryPolicies::cosine_similarity(&a, &b);
                let sim_ba = MemoryPolicies::cosine_similarity(&b, &a);
                prop_assert!(
                    (sim_ab - sim_ba).abs() < 1e-5,
                    "sim(a,b)={} != sim(b,a)={}", sim_ab, sim_ba
                );
            }

            #[test]
            fn cosine_similarity_is_bounded(
                a in finite_vec(8),
                b in finite_vec(8),
            ) {
                let sim = MemoryPolicies::cosine_similarity(&a, &b);
                prop_assert!(
                    sim >= -1.0 - 1e-6 && sim <= 1.0 + 1e-6,
                    "cosine similarity {} is out of [-1, 1] bounds", sim
                );
            }

            #[test]
            fn content_hash_is_deterministic(s in ".*") {
                let h1 = MemoryPolicies::content_hash(&s);
                let h2 = MemoryPolicies::content_hash(&s);
                prop_assert_eq!(h1, h2, "Same input must always produce the same hash");
            }

            #[test]
            fn content_hash_is_valid_hex_sha256(s in ".*") {
                let hash = MemoryPolicies::content_hash(&s);
                prop_assert_eq!(hash.len(), 64, "SHA-256 hex should be 64 chars");
                prop_assert!(
                    hash.chars().all(|c| c.is_ascii_hexdigit()),
                    "Hash should only contain hex digits: {}", hash
                );
            }

            #[test]
            fn cosine_self_similarity_is_one(
                v in finite_vec(4).prop_filter(
                    "non-zero vector",
                    |v| v.iter().any(|x| *x != 0.0)
                ),
            ) {
                let sim = MemoryPolicies::cosine_similarity(&v, &v);
                prop_assert!(
                    (sim - 1.0).abs() < 1e-4,
                    "Self-similarity should be ~1.0, got {}", sim
                );
            }

            #[test]
            fn decay_score_is_non_negative(
                base in 0.0_f32..10.0_f32,
                days in 0.0_f64..1000.0_f64,
            ) {
                let policies = MemoryPolicies::default();
                let score = policies.decay_score(base, days);
                prop_assert!(score >= 0.0, "Decay score should be non-negative, got {}", score);
            }

            #[test]
            fn decay_score_decreases_over_time(
                base in 0.01_f32..10.0_f32,
                day1 in 0.0_f64..500.0_f64,
            ) {
                let day2 = day1 + 1.0;
                let policies = MemoryPolicies::default();
                let score1 = policies.decay_score(base, day1);
                let score2 = policies.decay_score(base, day2);
                prop_assert!(
                    score1 >= score2,
                    "Score should not increase over time: day {} -> {}, day {} -> {}",
                    day1, score1, day2, score2
                );
            }
        }
    }
}
