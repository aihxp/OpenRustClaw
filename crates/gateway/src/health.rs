//! Health check endpoints for the OpenRustClaw gateway.
//!
//! Provides three levels of health checks:
//! - `/health` - Basic liveness check
//! - `/health/ready` - Readiness check (dependencies available)
//! - `/health/deep` - Deep health check (all subsystems)
//!
//! # Example Responses
//!
//! ## Basic Health (200 OK)
//! ```json
//! {
//!   "status": "healthy",
//!   "service": "openrustclaw-gateway",
//!   "version": "0.1.0",
//!   "timestamp": "2024-01-15T10:30:00Z"
//! }
//! ```
//!
//! ## Readiness Check (200 OK)
//! ```json
//! {
//!   "status": "ready",
//!   "checks": {
//!     "database": { "status": "healthy", "latency_ms": 5 },
//!     "providers": { "status": "healthy", "available": 3 }
//!   }
//! }
//! ```
//!
//! ## Deep Health (200 OK or 503 if degraded)
//! ```json
//! {
//!   "status": "healthy",
//!   "components": {
//!     "database": { "status": "healthy" },
//!     "memory": { "status": "healthy" },
//!     "providers": {
//!       "status": "healthy",
//!       "details": {
//!         "anthropic": "available",
//!         "openai": "available"
//!       }
//!     },
//!     "scheduler": { "status": "healthy" }
//!   }
//! }
//! ```

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::get,
    Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, error, instrument, warn};

/// Health check status variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    /// Service is healthy/ready.
    Healthy,
    /// Service is experiencing issues but is functional.
    Degraded,
    /// Service is unhealthy/not ready.
    Unhealthy,
}

impl HealthStatus {
    /// Returns the HTTP status code appropriate for this health status.
    pub fn http_status(&self) -> StatusCode {
        match self {
            HealthStatus::Healthy => StatusCode::OK,
            HealthStatus::Degraded => StatusCode::OK, // Still serving traffic
            HealthStatus::Unhealthy => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    /// Merge multiple statuses into one.
    /// Returns Unhealthy if any is unhealthy, Degraded if any is degraded, Healthy otherwise.
    pub fn merge(statuses: &[HealthStatus]) -> Self {
        if statuses.iter().any(|s| *s == HealthStatus::Unhealthy) {
            HealthStatus::Unhealthy
        } else if statuses.iter().any(|s| *s == HealthStatus::Degraded) {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        }
    }
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "healthy"),
            HealthStatus::Degraded => write!(f, "degraded"),
            HealthStatus::Unhealthy => write!(f, "unhealthy"),
        }
    }
}

/// Individual component health check result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    /// The health status of this component.
    pub status: HealthStatus,
    /// Optional details about the component.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    /// Optional error message if unhealthy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Response time for this check in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
}

impl ComponentHealth {
    /// Create a healthy component result.
    pub fn healthy() -> Self {
        Self {
            status: HealthStatus::Healthy,
            details: None,
            error: None,
            latency_ms: None,
        }
    }

    /// Create a healthy component with details.
    pub fn healthy_with_details(details: serde_json::Value) -> Self {
        Self {
            status: HealthStatus::Healthy,
            details: Some(details),
            error: None,
            latency_ms: None,
        }
    }

    /// Create an unhealthy component with an error.
    pub fn unhealthy(error: impl Into<String>) -> Self {
        Self {
            status: HealthStatus::Unhealthy,
            details: None,
            error: Some(error.into()),
            latency_ms: None,
        }
    }

    /// Create a degraded component.
    pub fn degraded(message: impl Into<String>) -> Self {
        Self {
            status: HealthStatus::Degraded,
            details: Some(serde_json::json!({ "message": message.into() })),
            error: None,
            latency_ms: None,
        }
    }

    /// Add latency information.
    pub fn with_latency(mut self, latency_ms: u64) -> Self {
        self.latency_ms = Some(latency_ms);
        self
    }
}

/// Basic health check response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Overall health status.
    pub status: HealthStatus,
    /// Service name.
    pub service: &'static str,
    /// Service version.
    pub version: &'static str,
    /// Timestamp of the check.
    pub timestamp: String,
    /// Uptime in seconds (if available).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uptime_seconds: Option<u64>,
}

/// Readiness check response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessResponse {
    /// Overall readiness status.
    pub status: HealthStatus,
    /// Individual check results.
    pub checks: HashMap<String, ComponentHealth>,
    /// Timestamp of the check.
    pub timestamp: String,
}

/// Deep health check response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepHealthResponse {
    /// Overall health status.
    pub status: HealthStatus,
    /// Component health details.
    pub components: HashMap<String, ComponentHealth>,
    /// Timestamp of the check.
    pub timestamp: String,
    /// Total check duration in milliseconds.
    pub duration_ms: u64,
}

/// Trait for health check implementations.
#[async_trait::async_trait]
pub trait HealthCheck: Send + Sync {
    /// The name of this health check.
    fn name(&self) -> &str;

    /// Perform the health check.
    async fn check(&self) -> ComponentHealth;
}

/// Registry of health checks.
pub struct HealthCheckRegistry {
    checks: Vec<Box<dyn HealthCheck>>,
    start_time: Instant,
}

impl Default for HealthCheckRegistry {
    fn default() -> Self {
        Self {
            checks: Vec::new(),
            start_time: Instant::now(),
        }
    }
}

impl HealthCheckRegistry {
    /// Create a new health check registry.
    pub fn new() -> Self {
        Self {
            checks: Vec::new(),
            start_time: Instant::now(),
        }
    }

    /// Register a health check.
    pub fn register(&mut self, check: Box<dyn HealthCheck>) {
        self.checks.push(check);
    }

    /// Register a simple health check from a closure.
    pub fn register_fn<F, Fut>(
        &mut self,
        name: impl Into<String>,
        check_fn: F,
    ) where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ComponentHealth> + Send + 'static,
    {
        struct FnHealthCheck<F> {
            name: String,
            f: F,
        }

        #[async_trait::async_trait]
        impl<F, Fut> HealthCheck for FnHealthCheck<F>
        where
            F: Fn() -> Fut + Send + Sync,
            Fut: std::future::Future<Output = ComponentHealth> + Send,
        {
            fn name(&self) -> &str {
                &self.name
            }

            async fn check(&self) -> ComponentHealth {
                (self.f)().await
            }
        }

        self.checks.push(Box::new(FnHealthCheck {
            name: name.into(),
            f: check_fn,
        }));
    }

    /// Get uptime in seconds.
    pub fn uptime_seconds(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    /// Run all registered checks.
    pub async fn run_checks(&self) -> HashMap<String, ComponentHealth> {
        let mut results = HashMap::new();

        for check in &self.checks {
            let start = Instant::now();
            let mut result = check.check().await;
            result.latency_ms = Some(start.elapsed().as_millis() as u64);
            results.insert(check.name().to_string(), result);
        }

        results
    }

    /// Get the overall status from check results.
    pub fn overall_status(&self, results: &HashMap<String, ComponentHealth>) -> HealthStatus {
        let statuses: Vec<_> = results.values().map(|r| r.status).collect();
        HealthStatus::merge(&statuses)
    }
}

impl std::fmt::Debug for HealthCheckRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HealthCheckRegistry")
            .field("checks_count", &self.checks.len())
            .field("uptime_seconds", &self.uptime_seconds())
            .finish()
    }
}

/// Shared state for health check handlers.
#[derive(Clone)]
pub struct HealthState {
    pub registry: Arc<HealthCheckRegistry>,
    pub db_pool: Option<openrustclaw_db::SqlitePool>,
    pub provider_health: Arc<dyn Fn() -> HashMap<String, String> + Send + Sync>,
}

impl HealthState {
    /// Create a new health state.
    pub fn new(registry: Arc<HealthCheckRegistry>) -> Self {
        Self {
            registry,
            db_pool: None,
            provider_health: Arc::new(|| HashMap::new()),
        }
    }

    /// Set the database pool.
    pub fn with_db_pool(mut self, pool: openrustclaw_db::SqlitePool) -> Self {
        self.db_pool = Some(pool);
        self
    }

    /// Set the provider health check function.
    pub fn with_provider_health<F>(mut self, f: F) -> Self
    where
        F: Fn() -> HashMap<String, String> + Send + Sync + 'static,
    {
        self.provider_health = Arc::new(f);
        self
    }
}

impl std::fmt::Debug for HealthState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HealthState")
            .field("has_db_pool", &self.db_pool.is_some())
            .field("registry", &self.registry)
            .finish()
    }
}

/// Create a router with health check endpoints.
pub fn health_routes() -> Router<HealthState> {
    Router::new()
        .route("/health", get(basic_health_handler))
        .route("/health/ready", get(readiness_handler))
        .route("/health/deep", get(deep_health_handler))
}

/// Basic liveness check handler.
///
/// Returns 200 OK if the service is running.
/// This is a lightweight check that doesn't verify dependencies.
#[instrument(skip(state))]
async fn basic_health_handler(State(state): State<HealthState>) -> impl IntoResponse {
    debug!("Handling basic health check");

    let response = HealthResponse {
        status: HealthStatus::Healthy,
        service: "openrustclaw-gateway",
        version: env!("CARGO_PKG_VERSION"),
        timestamp: Utc::now().to_rfc3339(),
        uptime_seconds: Some(state.registry.uptime_seconds()),
    };

    (StatusCode::OK, Json(response))
}

/// Readiness check handler.
///
/// Verifies that required dependencies are available:
/// - Database connectivity
/// - LLM provider availability
///
/// Returns 200 if ready, 503 if not ready.
#[instrument(skip(state))]
async fn readiness_handler(State(state): State<HealthState>) -> impl IntoResponse {
    debug!("Handling readiness check");
    let _start = Instant::now();

    let mut checks = HashMap::new();

    // Check database
    if let Some(ref pool) = state.db_pool {
        let db_start = Instant::now();
        match check_database(pool).await {
            Ok(()) => {
                let latency = db_start.elapsed().as_millis() as u64;
                checks.insert(
                    "database".to_string(),
                    ComponentHealth::healthy().with_latency(latency),
                );
            }
            Err(e) => {
                warn!(error = %e, "Database health check failed");
                checks.insert("database".to_string(), ComponentHealth::unhealthy(e.to_string()));
            }
        }
    } else {
        checks.insert(
            "database".to_string(),
            ComponentHealth::unhealthy("Database pool not configured"),
        );
    }

    // Check providers
    let provider_start = Instant::now();
    let provider_health = (state.provider_health)();
    let available_providers = provider_health
        .values()
        .filter(|v| *v == "available")
        .count();

    if available_providers > 0 {
        let latency = provider_start.elapsed().as_millis() as u64;
        checks.insert(
            "providers".to_string(),
            ComponentHealth::healthy_with_details(serde_json::json!({
                "available": available_providers,
                "details": provider_health
            }))
            .with_latency(latency),
        );
    } else {
        checks.insert(
            "providers".to_string(),
            ComponentHealth::unhealthy("No providers available"),
        );
    }

    let overall_status = state.registry.overall_status(&checks);

    let response = ReadinessResponse {
        status: overall_status,
        checks,
        timestamp: Utc::now().to_rfc3339(),
    };

    let status_code = overall_status.http_status();
    (status_code, Json(response))
}

/// Deep health check handler.
///
/// Performs comprehensive checks on all subsystems:
/// - Database (detailed connectivity and performance)
/// - Memory system
/// - All LLM providers (individual status)
/// - Scheduler
/// - Other configured checks
///
/// Returns detailed information about each component.
#[instrument(skip(state))]
async fn deep_health_handler(State(state): State<HealthState>) -> impl IntoResponse {
    debug!("Handling deep health check");
    let start = Instant::now();

    // Run all registered checks
    let mut components = state.registry.run_checks().await;

    // Add database check with more detail
    if let Some(ref pool) = state.db_pool {
        let db_start = Instant::now();
        match check_database_detailed(pool).await {
            Ok(details) => {
                let latency = db_start.elapsed().as_millis() as u64;
                components.insert(
                    "database".to_string(),
                    ComponentHealth::healthy_with_details(details).with_latency(latency),
                );
            }
            Err(e) => {
                error!(error = %e, "Database detailed health check failed");
                components.insert(
                    "database".to_string(),
                    ComponentHealth::unhealthy(e.to_string()),
                );
            }
        }
    }

    // Add memory check
    match check_memory_system().await {
        Ok(details) => {
            components.insert(
                "memory".to_string(),
                ComponentHealth::healthy_with_details(details),
            );
        }
        Err(e) => {
            components.insert(
                "memory".to_string(),
                ComponentHealth::degraded(format!("Memory check warning: {e}")),
            );
        }
    }

    // Add provider check with individual provider status
    let provider_health = (state.provider_health)();
    let available_count = provider_health
        .values()
        .filter(|v| *v == "available")
        .count();
    let total_count = provider_health.len();

    let provider_status = if available_count == 0 {
        HealthStatus::Unhealthy
    } else if available_count < total_count {
        HealthStatus::Degraded
    } else {
        HealthStatus::Healthy
    };

    components.insert(
        "providers".to_string(),
        ComponentHealth {
            status: provider_status,
            details: Some(serde_json::json!({
                "available": available_count,
                "total": total_count,
                "details": provider_health
            })),
            error: None,
            latency_ms: None,
        },
    );

    // Add scheduler check
    components.insert(
        "scheduler".to_string(),
        check_scheduler().await,
    );

    let overall_status = state.registry.overall_status(&components);
    let duration_ms = start.elapsed().as_millis() as u64;

    let response = DeepHealthResponse {
        status: overall_status,
        components,
        timestamp: Utc::now().to_rfc3339(),
        duration_ms,
    };

    let status_code = overall_status.http_status();
    (status_code, Json(response))
}

/// Basic database connectivity check.
async fn check_database(pool: &openrustclaw_db::SqlitePool) -> anyhow::Result<()> {
    // Perform a simple query to verify connectivity
    sqlx::query("SELECT 1")
        .fetch_one(pool)
        .await?;
    Ok(())
}

/// Detailed database health check.
async fn check_database_detailed(
    pool: &openrustclaw_db::SqlitePool,
) -> anyhow::Result<serde_json::Value> {
    let start = Instant::now();

    // Check connectivity
    sqlx::query("SELECT 1").fetch_one(pool).await?;

    // Get pool stats if available
    let pool_stats = sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM sqlite_master")
        .fetch_one(pool)
        .await;

    let latency_ms = start.elapsed().as_millis() as u64;

    Ok(serde_json::json!({
        "connected": true,
        "latency_ms": latency_ms,
        "tables": pool_stats.map(|(count,)| count).unwrap_or(-1),
    }))
}

/// Check memory system status.
async fn check_memory_system() -> anyhow::Result<serde_json::Value> {
    // In a real implementation, this would check:
    // - Memory store connectivity
    // - Embedding provider availability
    // - Recent operation latency

    Ok(serde_json::json!({
        "status": "operational",
        "notes": "Memory system health check placeholder"
    }))
}

/// Check scheduler status.
async fn check_scheduler() -> ComponentHealth {
    // In a real implementation, this would check:
    // - Scheduler worker is running
    // - Job queue depth
    // - Recent job execution success rate

    ComponentHealth::healthy_with_details(serde_json::json!({
        "status": "operational",
        "notes": "Scheduler health check placeholder"
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_merge() {
        assert_eq!(
            HealthStatus::merge(&[HealthStatus::Healthy, HealthStatus::Healthy]),
            HealthStatus::Healthy
        );
        assert_eq!(
            HealthStatus::merge(&[HealthStatus::Healthy, HealthStatus::Degraded]),
            HealthStatus::Degraded
        );
        assert_eq!(
            HealthStatus::merge(&[HealthStatus::Healthy, HealthStatus::Unhealthy]),
            HealthStatus::Unhealthy
        );
        assert_eq!(
            HealthStatus::merge(&[HealthStatus::Degraded, HealthStatus::Unhealthy]),
            HealthStatus::Unhealthy
        );
    }

    #[test]
    fn test_health_status_http_code() {
        assert_eq!(HealthStatus::Healthy.http_status(), StatusCode::OK);
        assert_eq!(HealthStatus::Degraded.http_status(), StatusCode::OK);
        assert_eq!(
            HealthStatus::Unhealthy.http_status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
    }

    #[test]
    fn test_component_health_builder() {
        let healthy = ComponentHealth::healthy();
        assert_eq!(healthy.status, HealthStatus::Healthy);
        assert!(healthy.error.is_none());

        let unhealthy = ComponentHealth::unhealthy("connection failed");
        assert_eq!(unhealthy.status, HealthStatus::Unhealthy);
        assert_eq!(unhealthy.error, Some("connection failed".to_string()));

        let with_latency = ComponentHealth::healthy().with_latency(42);
        assert_eq!(with_latency.latency_ms, Some(42));
    }

    #[test]
    fn test_health_check_registry() {
        let mut registry = HealthCheckRegistry::new();
        assert_eq!(registry.checks.len(), 0);

        registry.register_fn("test_check", || async {
            ComponentHealth::healthy()
        });
        assert_eq!(registry.checks.len(), 1);

        // Test uptime
        assert!(registry.uptime_seconds() < 2);
    }

    #[tokio::test]
    async fn test_run_checks() {
        let mut registry = HealthCheckRegistry::new();
        registry.register_fn("check1", || async { ComponentHealth::healthy() });
        registry.register_fn("check2", || async { ComponentHealth::healthy() });

        let results = registry.run_checks().await;
        assert_eq!(results.len(), 2);
        assert!(results.contains_key("check1"));
        assert!(results.contains_key("check2"));
    }
}
