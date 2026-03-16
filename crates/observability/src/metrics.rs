//! Token usage, latency, and cost tracking.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Metrics collector for tracking token usage, latency, and costs.
#[derive(Debug, Clone)]
pub struct MetricsCollector {
    inner: Arc<MetricsInner>,
}

#[derive(Debug)]
struct MetricsInner {
    total_prompt_tokens: AtomicU64,
    total_completion_tokens: AtomicU64,
    total_requests: AtomicU64,
    total_errors: AtomicU64,
    // Cost tracked in micro-USD (millionths of a dollar) for precision
    total_cost_micro_usd: AtomicU64,
}

/// Snapshot of current metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub total_prompt_tokens: u64,
    pub total_completion_tokens: u64,
    pub total_tokens: u64,
    pub total_requests: u64,
    pub total_errors: u64,
    pub total_cost_usd: f64,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(MetricsInner {
                total_prompt_tokens: AtomicU64::new(0),
                total_completion_tokens: AtomicU64::new(0),
                total_requests: AtomicU64::new(0),
                total_errors: AtomicU64::new(0),
                total_cost_micro_usd: AtomicU64::new(0),
            }),
        }
    }

    /// Record a completed LLM request.
    pub fn record_request(
        &self,
        prompt_tokens: u64,
        completion_tokens: u64,
        cost_usd: Option<f64>,
    ) {
        self.inner
            .total_prompt_tokens
            .fetch_add(prompt_tokens, Ordering::Relaxed);
        self.inner
            .total_completion_tokens
            .fetch_add(completion_tokens, Ordering::Relaxed);
        self.inner.total_requests.fetch_add(1, Ordering::Relaxed);
        if let Some(cost) = cost_usd {
            let micro = (cost * 1_000_000.0) as u64;
            self.inner
                .total_cost_micro_usd
                .fetch_add(micro, Ordering::Relaxed);
        }
    }

    /// Record an error.
    pub fn record_error(&self) {
        self.inner.total_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Get a snapshot of current metrics.
    pub fn snapshot(&self) -> MetricsSnapshot {
        let prompt = self.inner.total_prompt_tokens.load(Ordering::Relaxed);
        let completion = self.inner.total_completion_tokens.load(Ordering::Relaxed);
        MetricsSnapshot {
            total_prompt_tokens: prompt,
            total_completion_tokens: completion,
            total_tokens: prompt + completion,
            total_requests: self.inner.total_requests.load(Ordering::Relaxed),
            total_errors: self.inner.total_errors.load(Ordering::Relaxed),
            total_cost_usd: self.inner.total_cost_micro_usd.load(Ordering::Relaxed) as f64
                / 1_000_000.0,
        }
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}
