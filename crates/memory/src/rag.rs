//! RAG pipeline: chunk, embed, retrieve, grade.

use openrustclaw_core::types::{ScoredMemory, SourceType};

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
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return vec![];
    }

    let mut chunks = Vec::new();
    let mut start = 0;

    while start < words.len() {
        let end = (start + chunk_size).min(words.len());
        let chunk = words[start..end].join(" ");
        chunks.push(chunk);

        if end >= words.len() {
            break;
        }

        start += chunk_size.saturating_sub(overlap);
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
