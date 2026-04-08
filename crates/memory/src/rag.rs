//! RAG pipeline: chunk, embed, retrieve, grade.

use openrustclaw_core::types::{RetrievalExplanation, ScoredMemory, SourceType};

/// Configuration for the RAG chunking pipeline.
#[derive(Debug, Clone)]
pub struct ChunkConfig {
    pub default_chunk_size: usize,
    pub chunk_overlap: usize,
    pub code_chunk_size: usize,
    pub doc_chunk_size: usize,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            default_chunk_size: 512,
            chunk_overlap: 128,
            code_chunk_size: 1024,
            doc_chunk_size: 512,
        }
    }
}

/// A document chunk ready for embedding.
#[derive(Debug, Clone)]
pub struct Chunk {
    pub content: String,
    pub source: String,
    pub source_type: SourceType,
    pub chunk_index: usize,
    pub metadata: serde_json::Value,
}

/// Split text into overlapping chunks.
pub fn chunk_text(text: &str, chunk_size: usize, overlap: usize) -> Vec<String> {
    if chunk_size == 0 {
        return vec![];
    }

    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return vec![];
    }

    let step = chunk_size.saturating_sub(overlap).max(1);
    let mut chunks = Vec::new();
    let mut start = 0;

    while start < words.len() {
        let end = (start + chunk_size).min(words.len());
        let chunk = words[start..end].join(" ");
        chunks.push(chunk);

        if end >= words.len() {
            break;
        }

        start += step;
    }

    chunks
}

/// Grade retrieval results by relevance threshold.
pub fn grade_results(results: Vec<ScoredMemory>, min_score: f32) -> Vec<ScoredMemory> {
    results
        .into_iter()
        .filter(|r| r.score >= min_score)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use openrustclaw_core::types::{MemoryEntry, MemoryType};
    use uuid::Uuid;

    fn make_scored_memory(content: &str, score: f32) -> ScoredMemory {
        ScoredMemory {
            entry: MemoryEntry {
                id: Uuid::new_v4(),
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

    // ── chunk_text tests ──

    #[test]
    fn chunk_text_returns_empty_for_zero_chunk_size() {
        assert!(chunk_text("alpha beta", 0, 0).is_empty());
    }

    #[test]
    fn chunk_text_progresses_when_overlap_matches_chunk_size() {
        let chunks = chunk_text("one two three four", 2, 2);
        assert_eq!(chunks, vec!["one two", "two three", "three four"]);
    }

    #[test]
    fn chunk_text_empty_input() {
        let chunks = chunk_text("", 10, 2);
        assert!(chunks.is_empty());
    }

    #[test]
    fn chunk_text_whitespace_only() {
        let chunks = chunk_text("   \t\n  ", 10, 2);
        assert!(chunks.is_empty());
    }

    #[test]
    fn chunk_text_single_word() {
        let chunks = chunk_text("hello", 5, 0);
        assert_eq!(chunks, vec!["hello"]);
    }

    #[test]
    fn chunk_text_no_overlap() {
        let chunks = chunk_text("a b c d e f", 2, 0);
        assert_eq!(chunks, vec!["a b", "c d", "e f"]);
    }

    #[test]
    fn chunk_text_with_overlap() {
        let chunks = chunk_text("a b c d e", 3, 1);
        // step = 3 - 1 = 2
        // chunk 0: a b c (start=0, end=3)
        // chunk 1: c d e (start=2, end=5)
        assert_eq!(chunks, vec!["a b c", "c d e"]);
    }

    #[test]
    fn chunk_text_chunk_larger_than_input() {
        let chunks = chunk_text("one two", 10, 0);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], "one two");
    }

    #[test]
    fn chunk_text_overlap_larger_than_chunk() {
        // When overlap >= chunk_size, step should be 1 (guaranteed progress)
        let chunks = chunk_text("a b c d", 2, 5);
        // step = max(2 - 5, 1) = max(-3, 1) = 1 (saturating_sub gives 0, then max(0,1) = 1)
        assert!(
            chunks.len() >= 3,
            "Should still make progress with large overlap"
        );
    }

    // ── grade_results tests ──

    #[test]
    fn grade_results_filters_below_threshold() {
        let results = vec![
            make_scored_memory("high", 0.9),
            make_scored_memory("low", 0.3),
            make_scored_memory("mid", 0.6),
        ];
        let graded = grade_results(results, 0.5);
        assert_eq!(graded.len(), 2);
        assert!(graded.iter().all(|r| r.score >= 0.5));
    }

    #[test]
    fn grade_results_zero_threshold_keeps_all() {
        let results = vec![make_scored_memory("a", 0.1), make_scored_memory("b", 0.01)];
        let graded = grade_results(results, 0.0);
        assert_eq!(graded.len(), 2);
    }

    #[test]
    fn grade_results_high_threshold_filters_all() {
        let results = vec![make_scored_memory("a", 0.5), make_scored_memory("b", 0.8)];
        let graded = grade_results(results, 0.99);
        assert!(graded.is_empty());
    }

    #[test]
    fn grade_results_empty_input() {
        let graded = grade_results(vec![], 0.5);
        assert!(graded.is_empty());
    }

    #[test]
    fn grade_results_exact_threshold_included() {
        let results = vec![make_scored_memory("exact", 0.5)];
        let graded = grade_results(results, 0.5);
        assert_eq!(
            graded.len(),
            1,
            "Score exactly at threshold should be included"
        );
    }

    // ── ChunkConfig tests ──

    #[test]
    fn chunk_config_defaults() {
        let config = ChunkConfig::default();
        assert_eq!(config.default_chunk_size, 512);
        assert_eq!(config.chunk_overlap, 128);
        assert_eq!(config.code_chunk_size, 1024);
        assert_eq!(config.doc_chunk_size, 512);
    }
}
