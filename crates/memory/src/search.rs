//! Hybrid search: BM25 + vector similarity fusion.

use openrustclaw_core::types::ScoredMemory;

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
fn content_overlap(a: &str, b: &str) -> f32 {
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
