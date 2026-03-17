//! Embedding utilities for text and documents

use crate::{Content, Embedding, GeminiClient, GeminiError};
use async_trait::async_trait;

/// Builder for embedding requests
pub struct EmbedBuilder<'a> {
    client: &'a GeminiClient,
    output_dimensionality: Option<i32>,
}

impl<'a> EmbedBuilder<'a> {
    /// Create new embed builder
    pub fn new(client: &'a GeminiClient) -> Self {
        Self {
            client,
            output_dimensionality: None,
        }
    }

    /// Set output dimensionality
    pub fn with_dimensionality(mut self, dims: i32) -> Self {
        self.output_dimensionality = Some(dims);
        self
    }

    /// Embed a single text
    pub async fn embed_text(&self, text: impl Into<String>) -> Result<Embedding, GeminiError> {
        let content = Content::user(text);
        self.client
            .embed_content(content, self.output_dimensionality)
            .await
    }

    /// Embed multiple texts
    pub async fn embed_texts(&self, texts: Vec<String>) -> Result<Vec<Embedding>, GeminiError> {
        let contents: Vec<Content> = texts.into_iter().map(Content::user).collect();
        self.client
            .batch_embed_contents(contents, self.output_dimensionality)
            .await
    }

    /// Embed a document with title
    pub async fn embed_document(
        &self,
        title: &str,
        text: impl Into<String>,
    ) -> Result<Embedding, GeminiError> {
        let content_text = format!("Title: {}\n\n{}", title, text.into());
        let content = Content::user(content_text);
        self.client
            .embed_content(content, self.output_dimensionality)
            .await
    }
}

/// Calculate cosine similarity between two embeddings
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot_product / (norm_a * norm_b)
}

/// Find top-k most similar embeddings
pub fn find_similar<'a>(
    query: &[f32],
    embeddings: &'a [(String, Vec<f32>)],
    k: usize,
) -> Vec<(usize, &'a str, f32)> {
    let mut similarities: Vec<(usize, &str, f32)> = embeddings
        .iter()
        .enumerate()
        .map(|(idx, (text, emb))| (idx, text.as_str(), cosine_similarity(query, emb)))
        .collect();

    // Sort by similarity (descending)
    similarities.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

    // Take top k
    similarities.into_iter().take(k).collect()
}

/// Normalize an embedding vector
pub fn normalize(embedding: &mut [f32]) {
    let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in embedding.iter_mut() {
            *x /= norm;
        }
    }
}

/// Extension trait for GeminiClient to add embedding helpers
#[async_trait]
pub trait EmbeddingExt {
    /// Create embed builder
    fn embed_builder(&self) -> EmbedBuilder<'_>;

    /// Quick embed text
    async fn embed_text_simple(
        &self,
        text: impl Into<String> + Send,
    ) -> Result<Embedding, GeminiError>;
}

#[async_trait]
impl EmbeddingExt for GeminiClient {
    fn embed_builder(&self) -> EmbedBuilder<'_> {
        EmbedBuilder::new(self)
    }

    async fn embed_text_simple(
        &self,
        text: impl Into<String> + Send,
    ) -> Result<Embedding, GeminiError> {
        let content = Content::user(text);
        self.embed_content(content, None).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let c = vec![1.0, 0.0, 0.0];

        assert!((cosine_similarity(&a, &b) - 0.0).abs() < 0.001);
        assert!((cosine_similarity(&a, &c) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_normalize() {
        let mut v = vec![3.0, 4.0];
        normalize(&mut v);
        let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_find_similar() {
        let embeddings = vec![
            ("doc1".to_string(), vec![1.0, 0.0, 0.0]),
            ("doc2".to_string(), vec![0.0, 1.0, 0.0]),
            ("doc3".to_string(), vec![0.9, 0.1, 0.0]),
        ];

        let query = vec![1.0, 0.0, 0.0];
        let results = find_similar(&query, &embeddings, 2);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, 0); // doc1 is most similar
    }
}
