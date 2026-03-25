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
        let _ = self.inner.active_connections.fetch_update(
            Ordering::Relaxed,
            Ordering::Relaxed,
            |count| count.checked_sub(1),
        );
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
    metrics::counter!(
        "openrustclaw_requests_total",
        "method" => method.to_string(),
        "endpoint" => endpoint.to_string(),
        "status" => status.to_string()
    )
    .increment(1);
}

/// Record request duration.
///
/// # Arguments
/// * `method` - HTTP method or "websocket"
/// * `endpoint` - Request path
/// * `duration_secs` - Duration in seconds
pub fn record_request_duration(method: &str, endpoint: &str, duration_secs: f64) {
    metrics::histogram!(
        "openrustclaw_request_duration_seconds",
        "method" => method.to_string(),
        "endpoint" => endpoint.to_string()
    )
    .record(duration_secs);
}

/// Record WebSocket message.
///
/// # Arguments
/// * `direction` - "in" or "out"
/// * `msg_type" - "text", "binary", "ping", "pong", "close"
pub fn record_websocket_message(direction: &str, msg_type: &str) {
    metrics::counter!(
        "openrustclaw_websocket_messages_total",
        "direction" => direction.to_string(),
        "type" => msg_type.to_string()
    )
    .increment(1);
}

/// Update active WebSocket connections gauge.
pub fn set_active_connections(count: usize) {
    global_collector().set_active_connections(count);
    metrics::gauge!("openrustclaw_active_connections").set(count as f64);
}

/// Increment active connections.
pub fn increment_active_connections() {
    global_collector().increment_active_connections();
    let count = global_collector()
        .inner
        .active_connections
        .load(Ordering::Relaxed);
    metrics::gauge!("openrustclaw_active_connections").set(count as f64);
}

/// Decrement active connections.
pub fn decrement_active_connections() {
    global_collector().decrement_active_connections();
    let count = global_collector()
        .inner
        .active_connections
        .load(Ordering::Relaxed);
    metrics::gauge!("openrustclaw_active_connections").set(count as f64);
}

/// Record rate limit hit.
///
/// # Arguments
/// * `endpoint` - The endpoint that was rate limited
/// * `client_id" - Optional client identifier
pub fn record_rate_limit_hit(endpoint: &str, client_id: Option<&str>) {
    metrics::counter!(
        "openrustclaw_rate_limits_total",
        "endpoint" => endpoint.to_string(),
        "client" => client_id.unwrap_or("unknown").to_string()
    )
    .increment(1);
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
    metrics::counter!(
        "openrustclaw_provider_requests_total",
        "provider" => provider.to_string(),
        "model" => model.to_string(),
        "status" => status.to_string()
    )
    .increment(1);
}

/// Record LLM provider request duration.
///
/// # Arguments
/// * `provider` - Provider name
/// * `model" - Model identifier
/// * `duration_secs" - Duration in seconds
pub fn record_provider_duration(provider: &str, model: &str, duration_secs: f64) {
    metrics::histogram!(
        "openrustclaw_provider_duration_seconds",
        "provider" => provider.to_string(),
        "model" => model.to_string()
    )
    .record(duration_secs);
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
    metrics::counter!(
        "openrustclaw_tokens_total",
        "provider" => provider.to_string(),
        "model" => model.to_string(),
        "type" => "prompt"
    )
    .increment(prompt_tokens);
    metrics::counter!(
        "openrustclaw_tokens_total",
        "provider" => provider.to_string(),
        "model" => model.to_string(),
        "type" => "completion"
    )
    .increment(completion_tokens);
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
    metrics::histogram!(
        "openrustclaw_request_cost_usd",
        "provider" => provider.to_string(),
        "model" => model.to_string()
    )
    .record(cost_usd);
}

/// Record provider error.
///
/// # Arguments
/// * `provider` - Provider name
/// * `error_type" - Type of error (e.g., "rate_limited", "auth_failed", "timeout")
pub fn record_provider_error(provider: &str, error_type: &str) {
    global_collector().record_error();
    metrics::counter!(
        "openrustclaw_provider_errors_total",
        "provider" => provider.to_string(),
        "error_type" => error_type.to_string()
    )
    .increment(1);
}

/// Record streaming chunk.
///
/// # Arguments
/// * `provider` - Provider name
/// * `chunk_type" - Type of chunk ("content", "tool_call")
pub fn record_streaming_chunk(provider: &str, chunk_type: &str) {
    metrics::counter!(
        "openrustclaw_streaming_chunks_total",
        "provider" => provider.to_string(),
        "type" => chunk_type.to_string()
    )
    .increment(1);
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
    metrics::counter!(
        "openrustclaw_tool_executions_total",
        "tool" => tool_name.to_string(),
        "status" => status.to_string()
    )
    .increment(1);
}

/// Record tool execution duration.
///
/// # Arguments
/// * `tool_name` - Name of the tool
/// * `duration_secs" - Duration in seconds
pub fn record_tool_duration(tool_name: &str, duration_secs: f64) {
    metrics::histogram!(
        "openrustclaw_tool_duration_seconds",
        "tool" => tool_name.to_string()
    )
    .record(duration_secs);
}

/// Record agent processing iteration.
///
/// # Arguments
/// * `agent_name` - Name of the agent
/// * `finish_reason` - Why the agent stopped ("stop", "tool_use", "max_tokens")
pub fn record_agent_iteration(agent_name: &str, finish_reason: &str) {
    metrics::counter!(
        "openrustclaw_agent_iterations_total",
        "agent" => agent_name.to_string(),
        "finish_reason" => finish_reason.to_string()
    )
    .increment(1);
}

/// Record number of tool calls in an agent session.
///
/// # Arguments
/// * `agent_name` - Name of the agent
/// * `tool_calls" - Number of tool calls made
pub fn record_agent_tool_calls(agent_name: &str, tool_calls: u64) {
    metrics::histogram!(
        "openrustclaw_agent_tool_calls",
        "agent" => agent_name.to_string()
    )
    .record(tool_calls as f64);
}

/// Record agent session start.
pub fn record_agent_session_start(agent_name: &str) {
    metrics::counter!(
        "openrustclaw_agent_sessions_total",
        "agent" => agent_name.to_string(),
        "event" => "start"
    )
    .increment(1);
}

/// Record agent session end.
pub fn record_agent_session_end(agent_name: &str) {
    metrics::counter!(
        "openrustclaw_agent_sessions_total",
        "agent" => agent_name.to_string(),
        "event" => "end"
    )
    .increment(1);
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
    metrics::counter!(
        "openrustclaw_memory_operations_total",
        "operation" => operation.to_string(),
        "memory_type" => memory_type.to_string(),
        "status" => status.to_string()
    )
    .increment(1);
}

/// Record memory operation duration.
///
/// # Arguments
/// * `operation` - Operation type
/// * `duration_secs" - Duration in seconds
pub fn record_memory_duration(operation: &str, duration_secs: f64) {
    metrics::histogram!(
        "openrustclaw_memory_duration_seconds",
        "operation" => operation.to_string()
    )
    .record(duration_secs);
}

/// Record memory search results.
///
/// # Arguments
/// * `query_type` - Type of query
/// * `result_count" - Number of results returned
pub fn record_memory_search_results(query_type: &str, result_count: usize) {
    metrics::histogram!(
        "openrustclaw_memory_search_results",
        "query_type" => query_type.to_string()
    )
    .record(result_count as f64);
}

/// Record core memory size (in entries).
pub fn set_core_memory_entries(count: usize) {
    metrics::gauge!("openrustclaw_core_memory_entries").set(count as f64);
}

/// Record recall memory size (in entries).
pub fn set_recall_memory_entries(count: usize) {
    metrics::gauge!("openrustclaw_recall_memory_entries").set(count as f64);
}

/// Record memory maintenance activity.
pub fn record_memory_maintenance(operation: &str, status: &str, affected_count: usize) {
    metrics::counter!(
        "openrustclaw_memory_maintenance_total",
        "operation" => operation.to_string(),
        "status" => status.to_string()
    )
    .increment(1);
    metrics::histogram!(
        "openrustclaw_memory_maintenance_affected_entries",
        "operation" => operation.to_string(),
        "status" => status.to_string()
    )
    .record(affected_count as f64);
}

/// Record memory maintenance duration.
pub fn record_memory_maintenance_duration(operation: &str, duration_secs: f64) {
    metrics::histogram!(
        "openrustclaw_memory_maintenance_duration_seconds",
        "operation" => operation.to_string()
    )
    .record(duration_secs);
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
    metrics::counter!(
        "openrustclaw_cache_operations_total",
        "cache" => cache_name.to_string(),
        "operation" => operation.to_string()
    )
    .increment(1);
}

/// Set cache size.
///
/// # Arguments
/// * `cache_name` - Name of the cache
/// * `size" - Number of entries in cache
pub fn set_cache_size(cache_name: &str, size: usize) {
    metrics::gauge!(
        "openrustclaw_cache_size",
        "cache" => cache_name.to_string()
    )
    .set(size as f64);
}

/// Record cache eviction.
///
/// # Arguments
/// * `cache_name` - Name of the cache
/// * `reason` - Reason for eviction ("ttl", "capacity")
pub fn record_cache_eviction(cache_name: &str, reason: &str) {
    metrics::counter!(
        "openrustclaw_cache_evictions_total",
        "cache" => cache_name.to_string(),
        "reason" => reason.to_string()
    )
    .increment(1);
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
    metrics::counter!(
        "openrustclaw_db_queries_total",
        "query_type" => query_type.to_string(),
        "table" => table.to_string(),
        "status" => status.to_string()
    )
    .increment(1);
}

/// Record database query duration.
///
/// # Arguments
/// * `query_type` - Type of query
/// * `table` - Table name
/// * `duration_secs" - Duration in seconds
pub fn record_db_query_duration(query_type: &str, table: &str, duration_secs: f64) {
    metrics::histogram!(
        "openrustclaw_db_query_duration_seconds",
        "query_type" => query_type.to_string(),
        "table" => table.to_string()
    )
    .record(duration_secs);
}

/// Record database connection pool stats.
///
/// # Arguments
/// * `active" - Number of active connections
/// * `idle" - Number of idle connections
pub fn set_db_pool_stats(active: usize, idle: usize) {
    metrics::gauge!(
        "openrustclaw_db_pool_connections",
        "state" => "active"
    )
    .set(active as f64);
    metrics::gauge!(
        "openrustclaw_db_pool_connections",
        "state" => "idle"
    )
    .set(idle as f64);
}

/// Record database transaction.
///
/// # Arguments
/// * `operation` - "begin", "commit", "rollback"
pub fn record_db_transaction(operation: &str) {
    metrics::counter!(
        "openrustclaw_db_transactions_total",
        "operation" => operation.to_string()
    )
    .increment(1);
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
    metrics::counter!(
        "openrustclaw_job_executions_total",
        "job" => job_name.to_string(),
        "status" => status.to_string()
    )
    .increment(1);
}

/// Record job execution duration.
///
/// # Arguments
/// * `job_name` - Name of the job
/// * `duration_secs" - Duration in seconds
pub fn record_job_duration(job_name: &str, duration_secs: f64) {
    metrics::histogram!(
        "openrustclaw_job_duration_seconds",
        "job" => job_name.to_string()
    )
    .record(duration_secs);
}

/// Set number of scheduled jobs.
pub fn set_scheduled_jobs_count(count: usize) {
    metrics::gauge!("openrustclaw_scheduled_jobs").set(count as f64);
}

/// Set number of jobs by state.
///
/// # Arguments
/// * `state` - Job state ("active", "paused", "failed", "dead_letter")
/// * `count" - Number of jobs in this state
pub fn set_jobs_by_state(state: &str, count: usize) {
    metrics::gauge!(
        "openrustclaw_jobs_by_state",
        "state" => state.to_string()
    )
    .set(count as f64);
}

/// Record job retry.
///
/// # Arguments
/// * `job_name` - Name of the job
/// * `retry_count" - Current retry attempt number
pub fn record_job_retry(job_name: &str, retry_count: u32) {
    metrics::counter!(
        "openrustclaw_job_retries_total",
        "job" => job_name.to_string(),
        "retry_count" => retry_count.to_string()
    )
    .increment(1);
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
    metrics::counter!(
        "openrustclaw_auth_attempts_total",
        "method" => method.to_string(),
        "status" => status.to_string()
    )
    .increment(1);
}

/// Record origin validation.
///
/// # Arguments
/// * `status` - "allowed" or "denied"
pub fn record_origin_check(status: &str) {
    metrics::counter!(
        "openrustclaw_origin_checks_total",
        "status" => status.to_string()
    )
    .increment(1);
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
