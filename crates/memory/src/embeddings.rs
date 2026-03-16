//! Embedding provider abstraction.
//!
//! Wraps the EmbeddingProvider trait with concurrency limiting
//! to prevent CPU spikes (fixes OpenClaw's QMD crash issue).

use std::sync::Arc;

use openrustclaw_core::error::Result;
use openrustclaw_core::traits::EmbeddingProvider;
use tokio::sync::Semaphore;

/// Concurrency-limited embedding service.
pub struct EmbeddingService {
    provider: Arc<dyn EmbeddingProvider>,
    semaphore: Arc<Semaphore>,
}

impl EmbeddingService {
    pub fn new(provider: Arc<dyn EmbeddingProvider>, max_concurrent: usize) -> Self {
        Self {
            provider,
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
        }
    }

    /// Embed texts with concurrency limiting.
    pub async fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let _permit = self.semaphore.acquire().await.map_err(|e| {
            openrustclaw_core::error::Error::Memory(
                openrustclaw_core::error::MemoryError::Embedding(e.to_string()),
            )
        })?;
        self.provider.embed(texts).await
    }

    /// Get embedding dimensions.
    pub fn dimensions(&self) -> usize {
        self.provider.dimensions()
    }
}
