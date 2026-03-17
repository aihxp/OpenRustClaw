//! Prometheus metrics for OpenRustClaw.
//!
//! Provides counters, histograms, and gauges for:
//! - HTTP/WebSocket request tracking (gateway)
//! - LLM provider API calls and token usage
//! - Agent tool execution
//! - Memory operations
//! - Database query performance
//! - Cache hit/miss ratios
//! - Scheduler job metrics
//!
//! All metrics include appropriate labels for dimensional analysis.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Instant;

// ──────────────────────────────────────────────
// Legacy MetricsCollector (for backward compatibility)
// ──────────────────────────────────────────────

/// Metrics collector for tracking token usage, latency, and costs.
///
/// This is the legacy implementation. New code should use the metric functions below
/// which integrate with the Prometheus metrics system.
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
    active_connections: AtomicUsize,
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
    pub active_connections: usize,
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
                active_connections: AtomicUsize::new(0),
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
            active_connections: self.inner.active_connections.load(Ordering::Relaxed),
        }
    }

    /// Set the number of active connections.
    pub fn set_active_connections(&self, count: usize) {
        self.inner
            .active_connections
            .store(count, Ordering::Relaxed);
    }

    /// Increment active connections.
    pub fn increment_active_connections(&self) {
        self.inner
            .active_connections
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement active connections.
    pub fn decrement_active_connections(&self) {
        self.inner
            .active_connections
            .fetch_sub(1, Ordering::Relaxed);
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

// Global collector for use with standalone functions
use std::sync::OnceLock;
static GLOBAL_COLLECTOR: OnceLock<MetricsCollector> = OnceLock::new();

fn global_collector() -> &'static MetricsCollector {
    GLOBAL_COLLECTOR.get_or_init(MetricsCollector::new)
}

// ──────────────────────────────────────────────
// Simple Timer Utility
// ──────────────────────────────────────────────

/// A simple timer for recording durations.
pub struct SimpleTimer {
    start: Instant,
}

impl SimpleTimer {
    /// Create a new timer.
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    /// Get elapsed seconds.
    pub fn elapsed_secs(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }

    /// Get elapsed milliseconds.
    pub fn elapsed_ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }
}

impl Default for SimpleTimer {
    fn default() -> Self {
        Self::new()
    }
}

// ──────────────────────────────────────────────
// Metrics Initialization
// ──────────────────────────────────────────────

/// Initialize all OpenRustClaw metrics.
///
/// This should be called once at application startup.
pub fn init_metrics() {
    // The global collector is initialized lazily via get_or_init
    let _ = GLOBAL_COLLECTOR.get_or_init(MetricsCollector::new);
}

/// Get the global metrics collector.
pub fn get_collector() -> &'static MetricsCollector {
    global_collector()
}

// ──────────────────────────────────────────────
// Gateway Metrics
// ──────────────────────────────────────────────

/// Record an HTTP/WebSocket request.
///
/// # Arguments
/// * `method` - HTTP method or "websocket"
/// * `endpoint` - Request path
/// * `status` - Response status code as string
pub fn record_request(method: &str, endpoint: &str, status: &str) {
    global_collector()
        .inner
        .total_requests
        .fetch_add(1, Ordering::Relaxed);
    // When using metrics crate with prometheus exporter:
    // metrics::counter!("openrustclaw_requests_total", "method" => method, "endpoint" => endpoint, "status" => status).increment(1);
    let _ = (method, endpoint, status);
}

/// Record request duration.
///
/// # Arguments
/// * `method` - HTTP method or "websocket"
/// * `endpoint` - Request path
/// * `duration_secs` - Duration in seconds
pub fn record_request_duration(method: &str, endpoint: &str, duration_secs: f64) {
    // Placeholder for metrics crate integration
    let _ = (method, endpoint, duration_secs);
}

/// Record WebSocket message.
///
/// # Arguments
/// * `direction` - "in" or "out"
/// * `msg_type" - "text", "binary", "ping", "pong", "close"
pub fn record_websocket_message(direction: &str, msg_type: &str) {
    let _ = (direction, msg_type);
    // metrics::counter!("openrustclaw_websocket_messages_total", "direction" => direction, "type" => msg_type).increment(1);
}

/// Update active WebSocket connections gauge.
pub fn set_active_connections(count: usize) {
    global_collector().set_active_connections(count);
}

/// Increment active connections.
pub fn increment_active_connections() {
    global_collector().increment_active_connections();
}

/// Decrement active connections.
pub fn decrement_active_connections() {
    global_collector().decrement_active_connections();
}

/// Record rate limit hit.
///
/// # Arguments
/// * `endpoint` - The endpoint that was rate limited
/// * `client_id" - Optional client identifier
pub fn record_rate_limit_hit(endpoint: &str, client_id: Option<&str>) {
    let _ = (endpoint, client_id);
}

// ──────────────────────────────────────────────
// Provider Metrics
// ──────────────────────────────────────────────

/// Record an LLM provider API call.
///
/// # Arguments
/// * `provider` - Provider name (e.g., "anthropic", "openai")
/// * `model" - Model identifier
/// * `status" - "success" or "error"
pub fn record_provider_request(provider: &str, model: &str, status: &str) {
    let _ = (provider, model, status);
}

/// Record LLM provider request duration.
///
/// # Arguments
/// * `provider` - Provider name
/// * `model" - Model identifier
/// * `duration_secs" - Duration in seconds
pub fn record_provider_duration(provider: &str, model: &str, duration_secs: f64) {
    let _ = (provider, model, duration_secs);
}

/// Record token usage.
///
/// # Arguments
/// * `provider` - Provider name
/// * `model" - Model identifier
/// * `prompt_tokens" - Number of prompt tokens
/// * `completion_tokens" - Number of completion tokens
pub fn record_token_usage(provider: &str, model: &str, prompt_tokens: u64, completion_tokens: u64) {
    global_collector()
        .inner
        .total_prompt_tokens
        .fetch_add(prompt_tokens, Ordering::Relaxed);
    global_collector()
        .inner
        .total_completion_tokens
        .fetch_add(completion_tokens, Ordering::Relaxed);
    let _ = (provider, model);
}

/// Record estimated cost in USD (micro-cents for precision).
///
/// # Arguments
/// * `provider` - Provider name
/// * `model" - Model identifier
/// * `cost_usd" - Cost in USD
pub fn record_cost(provider: &str, model: &str, cost_usd: f64) {
    let micro = (cost_usd * 1_000_000.0) as u64;
    global_collector()
        .inner
        .total_cost_micro_usd
        .fetch_add(micro, Ordering::Relaxed);
    let _ = (provider, model);
}

/// Record provider error.
///
/// # Arguments
/// * `provider` - Provider name
/// * `error_type" - Type of error (e.g., "rate_limited", "auth_failed", "timeout")
pub fn record_provider_error(provider: &str, error_type: &str) {
    global_collector().record_error();
    let _ = (provider, error_type);
}

/// Record streaming chunk.
///
/// # Arguments
/// * `provider` - Provider name
/// * `chunk_type" - Type of chunk ("content", "tool_call")
pub fn record_streaming_chunk(provider: &str, chunk_type: &str) {
    let _ = (provider, chunk_type);
}

// ──────────────────────────────────────────────
// Agent Metrics
// ──────────────────────────────────────────────

/// Record tool execution.
///
/// # Arguments
/// * `tool_name` - Name of the tool
/// * `status" - "success" or "error"
pub fn record_tool_execution(tool_name: &str, status: &str) {
    let _ = (tool_name, status);
}

/// Record tool execution duration.
///
/// # Arguments
/// * `tool_name` - Name of the tool
/// * `duration_secs" - Duration in seconds
pub fn record_tool_duration(tool_name: &str, duration_secs: f64) {
    let _ = (tool_name, duration_secs);
}

/// Record agent processing iteration.
///
/// # Arguments
/// * `agent_name` - Name of the agent
/// * `finish_reason` - Why the agent stopped ("stop", "tool_use", "max_tokens")
pub fn record_agent_iteration(agent_name: &str, finish_reason: &str) {
    let _ = (agent_name, finish_reason);
}

/// Record number of tool calls in an agent session.
///
/// # Arguments
/// * `agent_name` - Name of the agent
/// * `tool_calls" - Number of tool calls made
pub fn record_agent_tool_calls(agent_name: &str, tool_calls: u64) {
    let _ = (agent_name, tool_calls);
}

/// Record agent session start.
pub fn record_agent_session_start(agent_name: &str) {
    let _ = agent_name;
}

/// Record agent session end.
pub fn record_agent_session_end(agent_name: &str) {
    let _ = agent_name;
}

// ──────────────────────────────────────────────
// Memory Metrics
// ──────────────────────────────────────────────

/// Record memory store operation.
///
/// # Arguments
/// * `operation` - Operation type ("store", "search", "update", "delete")
/// * `memory_type` - Type of memory ("episodic", "semantic", "procedural")
/// * `status" - "success" or "error"
pub fn record_memory_operation(operation: &str, memory_type: &str, status: &str) {
    let _ = (operation, memory_type, status);
}

/// Record memory operation duration.
///
/// # Arguments
/// * `operation` - Operation type
/// * `duration_secs" - Duration in seconds
pub fn record_memory_duration(operation: &str, duration_secs: f64) {
    let _ = (operation, duration_secs);
}

/// Record memory search results.
///
/// # Arguments
/// * `query_type` - Type of query
/// * `result_count" - Number of results returned
pub fn record_memory_search_results(query_type: &str, result_count: usize) {
    let _ = (query_type, result_count);
}

/// Record core memory size (in entries).
pub fn set_core_memory_entries(count: usize) {
    let _ = count;
}

/// Record recall memory size (in entries).
pub fn set_recall_memory_entries(count: usize) {
    let _ = count;
}

// ──────────────────────────────────────────────
// Cache Metrics
// ──────────────────────────────────────────────

/// Record cache operation.
///
/// # Arguments
/// * `cache_name` - Name of the cache
/// * `operation` - "hit" or "miss"
pub fn record_cache_operation(cache_name: &str, operation: &str) {
    let _ = (cache_name, operation);
}

/// Set cache size.
///
/// # Arguments
/// * `cache_name` - Name of the cache
/// * `size" - Number of entries in cache
pub fn set_cache_size(cache_name: &str, size: usize) {
    let _ = (cache_name, size);
}

/// Record cache eviction.
///
/// # Arguments
/// * `cache_name` - Name of the cache
/// * `reason` - Reason for eviction ("ttl", "capacity")
pub fn record_cache_eviction(cache_name: &str, reason: &str) {
    let _ = (cache_name, reason);
}

// ──────────────────────────────────────────────
// Database Metrics
// ──────────────────────────────────────────────

/// Record database query.
///
/// # Arguments
/// * `query_type` - Type of query ("select", "insert", "update", "delete")
/// * `table` - Table name
/// * `status" - "success" or "error"
pub fn record_db_query(query_type: &str, table: &str, status: &str) {
    let _ = (query_type, table, status);
}

/// Record database query duration.
///
/// # Arguments
/// * `query_type` - Type of query
/// * `table` - Table name
/// * `duration_secs" - Duration in seconds
pub fn record_db_query_duration(query_type: &str, table: &str, duration_secs: f64) {
    let _ = (query_type, table, duration_secs);
}

/// Record database connection pool stats.
///
/// # Arguments
/// * `active" - Number of active connections
/// * `idle" - Number of idle connections
pub fn set_db_pool_stats(active: usize, idle: usize) {
    let _ = (active, idle);
}

/// Record database transaction.
///
/// # Arguments
/// * `operation` - "begin", "commit", "rollback"
pub fn record_db_transaction(operation: &str) {
    let _ = operation;
}

// ──────────────────────────────────────────────
// Scheduler Metrics
// ──────────────────────────────────────────────

/// Record scheduler job execution.
///
/// # Arguments
/// * `job_name` - Name of the job
/// * `status" - "success", "failure", "timeout"
pub fn record_job_execution(job_name: &str, status: &str) {
    let _ = (job_name, status);
}

/// Record job execution duration.
///
/// # Arguments
/// * `job_name` - Name of the job
/// * `duration_secs" - Duration in seconds
pub fn record_job_duration(job_name: &str, duration_secs: f64) {
    let _ = (job_name, duration_secs);
}

/// Set number of scheduled jobs.
pub fn set_scheduled_jobs_count(count: usize) {
    let _ = count;
}

/// Set number of jobs by state.
///
/// # Arguments
/// * `state` - Job state ("active", "paused", "failed", "dead_letter")
/// * `count" - Number of jobs in this state
pub fn set_jobs_by_state(state: &str, count: usize) {
    let _ = (state, count);
}

/// Record job retry.
///
/// # Arguments
/// * `job_name` - Name of the job
/// * `retry_count" - Current retry attempt number
pub fn record_job_retry(job_name: &str, retry_count: u32) {
    let _ = (job_name, retry_count);
}

// ──────────────────────────────────────────────
// Security Metrics
// ──────────────────────────────────────────────

/// Record authentication attempt.
///
/// # Arguments
/// * `method` - Auth method ("token", "jwt")
/// * `status" - "success" or "failure"
pub fn record_auth_attempt(method: &str, status: &str) {
    let _ = (method, status);
}

/// Record origin validation.
///
/// # Arguments
/// * `status` - "allowed" or "denied"
pub fn record_origin_check(status: &str) {
    let _ = status;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_collector() {
        let collector = MetricsCollector::new();
        collector.record_request(100, 50, Some(0.001));

        let snapshot = collector.snapshot();
        assert_eq!(snapshot.total_prompt_tokens, 100);
        assert_eq!(snapshot.total_completion_tokens, 50);
        assert_eq!(snapshot.total_tokens, 150);
        assert_eq!(snapshot.total_requests, 1);
        assert!((snapshot.total_cost_usd - 0.001).abs() < 0.0001);
    }

    #[test]
    fn test_simple_timer() {
        let timer = SimpleTimer::new();
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(timer.elapsed_secs() >= 0.01);
    }

    #[test]
    fn test_active_connections() {
        let collector = MetricsCollector::new();

        collector.set_active_connections(10);
        assert_eq!(collector.snapshot().active_connections, 10);

        collector.increment_active_connections();
        assert_eq!(collector.snapshot().active_connections, 11);

        collector.decrement_active_connections();
        assert_eq!(collector.snapshot().active_connections, 10);
    }

    #[test]
    fn test_global_collector() {
        init_metrics();
        record_request("GET", "/test", "200");
        record_token_usage("test", "model", 100, 50);

        // These should not panic
        increment_active_connections();
        decrement_active_connections();
        set_active_connections(5);
    }
}
