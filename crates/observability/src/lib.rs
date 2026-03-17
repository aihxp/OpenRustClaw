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
    init_metrics, record_request, record_request_duration, record_websocket_message,
    set_active_connections, increment_active_connections, decrement_active_connections,
    record_rate_limit_hit, record_provider_request, record_provider_duration,
    record_token_usage, record_cost, record_provider_error, record_streaming_chunk,
    record_tool_execution, record_tool_duration, record_agent_iteration,
    record_agent_tool_calls, record_agent_session_start, record_agent_session_end,
    record_memory_operation, record_memory_duration, record_memory_search_results,
    set_core_memory_entries, set_recall_memory_entries, record_cache_operation,
    set_cache_size, record_cache_eviction, record_db_query, record_db_query_duration,
    set_db_pool_stats, record_db_transaction, record_job_execution, record_job_duration,
    set_scheduled_jobs_count, set_jobs_by_state, record_job_retry, record_auth_attempt,
    record_origin_check, SimpleTimer, MetricsCollector, MetricsSnapshot,
};

// Re-export tracing config types
pub use tracing_config::{
    init_tracing, init_tracing_json, init_tracing_pretty, init_tracing_test,
    Env, TraceContext, RequestId, request_span, request_span_with_id,
    extract_trace_context_from_headers, inject_trace_context_into_headers,
    RequestIdLayer,
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
