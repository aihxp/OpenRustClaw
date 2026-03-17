//! Comprehensive observability for OpenRustClaw.
//!
//! This crate provides:
//! - Prometheus metrics collection
//! - Structured logging with tracing
//! - Distributed tracing support
//! - LangSmith integration for LLM observability
//!
//! # Quick Start
//!
//! ```rust
//! use openrustclaw_observability::{
//!     init_metrics,
//!     tracing_config::{init_tracing, Env},
//!     metrics::*,
//! };
//!
//! fn main() {
//!     // Initialize tracing
//!     init_tracing(Env::detect());
//!     
//!     // Initialize metrics
//!     init_metrics();
//!     
//!     // Record metrics
//!     record_request("GET", "/health", "200");
//! }
//! ```

// Core modules
pub mod langsmith;
pub mod metrics;
pub mod tracing_config;

// Re-exports for convenience
pub use langsmith::LangSmithClient;

// Re-export all metrics functions
pub use metrics::{
    MetricsCollector, MetricsSnapshot, SimpleTimer, decrement_active_connections,
    increment_active_connections, init_metrics, record_agent_iteration, record_agent_session_end,
    record_agent_session_start, record_agent_tool_calls, record_auth_attempt,
    record_cache_eviction, record_cache_operation, record_cost, record_db_query,
    record_db_query_duration, record_db_transaction, record_job_duration, record_job_execution,
    record_job_retry, record_memory_duration, record_memory_operation,
    record_memory_search_results, record_origin_check, record_provider_duration,
    record_provider_error, record_provider_request, record_rate_limit_hit, record_request,
    record_request_duration, record_streaming_chunk, record_token_usage, record_tool_duration,
    record_tool_execution, record_websocket_message, set_active_connections, set_cache_size,
    set_core_memory_entries, set_db_pool_stats, set_jobs_by_state, set_recall_memory_entries,
    set_scheduled_jobs_count,
};

// Re-export tracing config types
pub use tracing_config::{
    Env, RequestId, RequestIdLayer, TraceContext, extract_trace_context_from_headers, init_tracing,
    init_tracing_json, init_tracing_pretty, init_tracing_test, inject_trace_context_into_headers,
    request_span, request_span_with_id,
};

/// Initialize all observability systems.
///
/// This is a convenience function that initializes both tracing and metrics.
///
/// # Example
///
/// ```rust
/// use openrustclaw_observability::init_observability;
///
/// fn main() {
///     init_observability();
///     // Your application code...
/// }
/// ```
pub fn init_observability() {
    tracing_config::init_tracing(Env::detect());
    metrics::init_metrics();
}

/// Initialize observability with explicit environment.
///
/// # Example
///
/// ```rust
/// use openrustclaw_observability::{init_observability_with_env, tracing_config::Env};
///
/// fn main() {
///     init_observability_with_env(Env::Production);
/// }
/// ```
pub fn init_observability_with_env(env: Env) {
    tracing_config::init_tracing(env);
    metrics::init_metrics();
}

/// Get current observability info.
///
/// Returns version and configuration information.
pub fn observability_info() -> serde_json::Value {
    serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "features": {
            "metrics": true,
            "tracing": true,
            "langsmith": true,
        }
    })
}
