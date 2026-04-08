//! Hybrid search: BM25 + vector similarity fusion.

use openrustclaw_core::types::{RetrievalExplanation, ScoredMemory};

/// Fuse BM25 and vector search results using Reciprocal Rank Fusion (RRF).
pub fn reciprocal_rank_fusion(
    bm25_results: Vec<ScoredMemory>,
    vector_results: Vec<ScoredMemory>,
    k: f32, // RRF constant, typically 60.0
) -> Vec<ScoredMemory> {
    use std::collections::HashMap;

    let mut scores: HashMap<String, (f32, Option<ScoredMemory>)> = HashMap::new();

    // Add BM25 scores
    for (rank, result) in bm25_results.into_iter().enumerate() {
        let rrf_score = 1.0 / (k + rank as f32 + 1.0);
        let entry = scores
            .entry(result.entry.id.to_string())
            .or_insert((0.0, None));
        entry.0 += rrf_score;
        entry.1 = Some(result);
    }

    // Add vector scores
    for (rank, result) in vector_results.into_iter().enumerate() {
        let rrf_score = 1.0 / (k + rank as f32 + 1.0);
        let entry = scores
            .entry(result.entry.id.to_string())
            .or_insert((0.0, None));
        entry.0 += rrf_score;
        if entry.1.is_none() {
            entry.1 = Some(result);
        }
    }

    let mut fused: Vec<ScoredMemory> = scores
        .into_values()
        .filter_map(|(score, result)| {
            result.map(|mut r| {
                r.score = score;
                r
            })
        })
        .collect();

    fused.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    fused
}

/// Apply Maximal Marginal Relevance (MMR) for diversity in results.
/// Lambda controls relevance vs diversity trade-off (0.5 = balanced).
pub fn mmr_rerank(results: Vec<ScoredMemory>, limit: usize, lambda: f32) -> Vec<ScoredMemory> {
    if results.len() <= limit {
        return results;
    }

    let mut selected: Vec<ScoredMemory> = Vec::with_capacity(limit);
    let mut remaining: Vec<ScoredMemory> = results;

    // Always select the top result first
    if let Some(first) = remaining.first().cloned() {
        selected.push(first);
        remaining.remove(0);
    }

    while selected.len() < limit && !remaining.is_empty() {
        let mut best_idx = 0;
        let mut best_mmr = f32::NEG_INFINITY;

        for (i, candidate) in remaining.iter().enumerate() {
            let relevance = candidate.score;

            // Max similarity to any already selected result
            // Since we don't have embeddings here, use content overlap as proxy
            let max_sim = selected
                .iter()
                .map(|s| content_overlap(&candidate.entry.content, &s.entry.content))
                .fold(0.0_f32, f32::max);

            let mmr_score = lambda * relevance - (1.0 - lambda) * max_sim;
            if mmr_score > best_mmr {
                best_mmr = mmr_score;
                best_idx = i;
            }
        }

        selected.push(remaining.remove(best_idx));
    }

    selected
}

/// Simple content overlap metric (Jaccard similarity of word sets).
/// Public for testing.
pub(crate) fn content_overlap(a: &str, b: &str) -> f32 {
    let words_a: std::collections::HashSet<&str> = a.split_whitespace().collect();
    let words_b: std::collections::HashSet<&str> = b.split_whitespace().collect();

    if words_a.is_empty() && words_b.is_empty() {
        return 1.0;
    }

    let intersection = words_a.intersection(&words_b).count();
    let union = words_a.union(&words_b).count();

    if union == 0 {
        0.0
    } else {
        intersection as f32 / union as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use openrustclaw_core::types::{MemoryEntry, MemoryType};
    use uuid::Uuid;

    fn make_scored(id: &str, content: &str, score: f32) -> ScoredMemory {
        ScoredMemory {
            entry: MemoryEntry {
                id: Uuid::parse_str(&format!("00000000-0000-0000-0000-{:0>12}", id))
                    .unwrap_or_else(|_| Uuid::new_v4()),
                memory_type: MemoryType::Semantic,
                content: content.to_string(),
                content_hash: String::new(),
                source: None,
                source_type: None,
                session_id: None,
                user_id: None,
                namespace: "global".to_string(),
                importance: 0.5,
                confidence: 1.0,
                access_count: 0,
                last_accessed: None,
                created_at: Utc::now(),
                expires_at: None,
                metadata: serde_json::Value::Object(serde_json::Map::new()),
            },
            score,
            explanation: RetrievalExplanation::default(),
        }
    }

    // ── content_overlap tests ──

    #[test]
    fn content_overlap_identical_strings() {
        let sim = content_overlap("hello world", "hello world");
        assert!(
            (sim - 1.0).abs() < 1e-6,
            "Identical strings should have overlap 1.0"
        );
    }

    #[test]
    fn content_overlap_completely_different() {
        let sim = content_overlap("apple banana", "cherry dragonfruit");
        assert_eq!(sim, 0.0, "No shared words should give overlap 0.0");
    }

    #[test]
    fn content_overlap_partial() {
        // "hello world" = {hello, world}
        // "hello there" = {hello, there}
        // intersection = 1, union = 3
        let sim = content_overlap("hello world", "hello there");
        assert!(
            (sim - 1.0 / 3.0).abs() < 1e-6,
            "Partial overlap should be 1/3"
        );
    }

    #[test]
    fn content_overlap_both_empty() {
        let sim = content_overlap("", "");
        assert_eq!(sim, 1.0, "Both empty strings should have overlap 1.0");
    }

    #[test]
    fn content_overlap_one_empty() {
        let sim = content_overlap("hello", "");
        // words_a = {hello}, words_b = {}
        // intersection = 0, union = 1
        assert_eq!(sim, 0.0);
    }

    // ── reciprocal_rank_fusion tests ──

    #[test]
    fn rrf_empty_inputs() {
        let result = reciprocal_rank_fusion(vec![], vec![], 60.0);
        assert!(result.is_empty());
    }

    #[test]
    fn rrf_only_bm25_results() {
        let bm25 = vec![
            make_scored("000000000001", "doc one", 1.0),
            make_scored("000000000002", "doc two", 0.8),
        ];
        let result = reciprocal_rank_fusion(bm25, vec![], 60.0);
        assert_eq!(result.len(), 2);
        // First result should have higher RRF score
        assert!(result[0].score > result[1].score);
    }

    #[test]
    fn rrf_only_vector_results() {
        let vector = vec![
            make_scored("000000000001", "vec one", 0.95),
            make_scored("000000000002", "vec two", 0.85),
        ];
        let result = reciprocal_rank_fusion(vec![], vector, 60.0);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn rrf_overlap_boosts_score() {
        // Same document appears in both lists, should get a higher combined score
        let bm25 = vec![make_scored("000000000001", "shared doc", 1.0)];
        let vector = vec![make_scored("000000000001", "shared doc", 0.9)];
        let combined = reciprocal_rank_fusion(bm25, vector, 60.0);
        assert_eq!(combined.len(), 1);
        // Score should be the sum of both RRF contributions
        let expected_score = 1.0 / (60.0 + 1.0) + 1.0 / (60.0 + 1.0);
        assert!((combined[0].score - expected_score).abs() < 1e-6);
    }

    #[test]
    fn rrf_sorted_descending() {
        let bm25 = vec![
            make_scored("000000000001", "doc a", 1.0),
            make_scored("000000000002", "doc b", 0.5),
        ];
        let vector = vec![
            make_scored("000000000002", "doc b", 0.9),
            make_scored("000000000003", "doc c", 0.4),
        ];
        let result = reciprocal_rank_fusion(bm25, vector, 60.0);
        for i in 1..result.len() {
            assert!(
                result[i - 1].score >= result[i].score,
                "Results must be sorted descending by score"
            );
        }
    }

    // ── mmr_rerank tests ──

    #[test]
    fn mmr_returns_all_when_under_limit() {
        let results = vec![
            make_scored("000000000001", "doc one", 0.9),
            make_scored("000000000002", "doc two", 0.8),
        ];
        let reranked = mmr_rerank(results, 5, 0.5);
        assert_eq!(reranked.len(), 2, "Should return all when count <= limit");
    }

    #[test]
    fn mmr_limits_output_count() {
        let results = vec![
            make_scored("000000000001", "doc one", 0.9),
            make_scored("000000000002", "doc two", 0.8),
            make_scored("000000000003", "doc three", 0.7),
            make_scored("000000000004", "doc four", 0.6),
        ];
        let reranked = mmr_rerank(results, 2, 0.5);
        assert_eq!(reranked.len(), 2, "Should limit to requested count");
    }

    #[test]
    fn mmr_first_result_is_top_scored() {
        let results = vec![
            make_scored("000000000001", "best result", 0.99),
            make_scored("000000000002", "second best", 0.90),
            make_scored("000000000003", "third best", 0.80),
        ];
        let reranked = mmr_rerank(results, 2, 0.5);
        assert_eq!(
            reranked[0].entry.content, "best result",
            "First selected should be highest scored"
        );
    }

    #[test]
    fn mmr_prefers_diversity() {
        // Two very similar docs and one different doc
        let results = vec![
            make_scored("000000000001", "rust programming language guide", 0.95),
            make_scored("000000000002", "rust programming language tutorial", 0.90),
            make_scored("000000000003", "python data science introduction", 0.85),
        ];
        let reranked = mmr_rerank(results, 2, 0.3); // Low lambda = prefer diversity
        // The second pick should prefer the diverse python doc over the similar rust doc
        assert_eq!(
            reranked[1].entry.content, "python data science introduction",
            "MMR with low lambda should prefer diverse results"
        );
    }

    #[test]
    fn mmr_empty_input() {
        let results: Vec<ScoredMemory> = vec![];
        let reranked = mmr_rerank(results, 5, 0.5);
        assert!(reranked.is_empty());
    }
}
