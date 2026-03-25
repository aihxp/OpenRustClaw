# Observability & Monitoring

This guide covers the comprehensive observability features built into OpenRustClaw, including metrics collection, structured logging, health checks, and monitoring dashboards.

## Overview

OpenRustClaw provides production-ready observability through:

- **Prometheus Metrics** - Dimensional metrics for all components
- **Structured Logging** - JSON/pretty formatted logs with request tracing
- **Health Checks** - Multi-level health endpoints for load balancers
- **Grafana Dashboards** - Pre-built visualizations for all metrics

## Quick Start

### 1. Initialize Observability

```rust
use openrustclaw_observability::{
    init_observability,
    tracing_config::Env,
};

fn main() {
    // Initialize with environment auto-detection
    init_observability();
    
    // Or specify explicitly
    // init_observability_with_env(Env::Production);
}
```

### 2. Configure Environment

```bash
# For JSON logs (production)
export RUST_ENV=production
export RUST_LOG=info,openrustclaw=debug

# For pretty logs (development)
export RUST_ENV=development
export RUST_LOG=debug
```

### 3. Start OpenRustClaw

```bash
openrustclaw start
```

The shipped runtime now exposes Prometheus metrics directly at:

```bash
curl http://localhost:8080/metrics
```

### 4. Start Prometheus and Grafana

```bash
# Start Prometheus
docker run -p 9090:9090 \
  -v ./prometheus/prometheus.yml:/etc/prometheus/prometheus.yml \
  prom/prometheus

# Start Grafana
docker run -p 3000:3000 \
  -v ./grafana/dashboard.json:/var/lib/grafana/dashboards/openrustclaw.json \
  grafana/grafana
```

## Metrics Reference

### Gateway Metrics

| Metric | Type | Description | Labels |
|--------|------|-------------|--------|
| `openrustclaw_requests_total` | Counter | HTTP/WebSocket requests | `method`, `endpoint`, `status` |
| `openrustclaw_request_duration_seconds` | Histogram | Request latency | `method`, `endpoint` |
| `openrustclaw_websocket_messages_total` | Counter | WebSocket messages | `direction`, `type` |
| `openrustclaw_active_connections` | Gauge | Active WebSocket connections | - |
| `openrustclaw_rate_limits_total` | Counter | Rate limit hits | `endpoint`, `client` |

### Provider Metrics

| Metric | Type | Description | Labels |
|--------|------|-------------|--------|
| `openrustclaw_provider_requests_total` | Counter | LLM API calls | `provider`, `model`, `status` |
| `openrustclaw_provider_duration_seconds` | Histogram | API latency | `provider`, `model` |
| `openrustclaw_tokens_total` | Counter | Token usage | `provider`, `model`, `type` |
| `openrustclaw_request_cost_usd` | Histogram | Estimated cost | `provider`, `model` |
| `openrustclaw_provider_errors_total` | Counter | Provider errors | `provider`, `error_type` |
| `openrustclaw_streaming_chunks_total` | Counter | Streaming chunks | `provider`, `type` |

### Agent Metrics

| Metric | Type | Description | Labels |
|--------|------|-------------|--------|
| `openrustclaw_tool_executions_total` | Counter | Tool executions | `tool`, `status` |
| `openrustclaw_tool_duration_seconds` | Histogram | Tool execution time | `tool` |
| `openrustclaw_agent_iterations_total` | Counter | Agent iterations | `agent`, `finish_reason` |
| `openrustclaw_agent_tool_calls` | Histogram | Tool calls per session | `agent` |
| `openrustclaw_agent_sessions_total` | Counter | Session starts/ends | `agent`, `event` |

The shipped control plane now records real operator-tool metrics for several Phase 8 parity lanes, including:

- `media.inspect`, `media.extract_text`, `media.describe`
- `skills.invoke`, `skills.execute`
- `skills.voice_plugin.prewarm`, `skills.voice_call.start|end|reconnect`
- `mobile.command.dispatch|approve|reject`
- `mobile.capability.execute`
- `channels.<platform>.ingress`
- `channels.<platform>.send`
- scheduler workflow/job runs via the existing `openrustclaw_job_executions_total` and `openrustclaw_job_duration_seconds` metrics

### Memory Metrics

| Metric | Type | Description | Labels |
|--------|------|-------------|--------|
| `openrustclaw_memory_operations_total` | Counter | Memory operations | `operation`, `memory_type`, `status` |
| `openrustclaw_memory_duration_seconds` | Histogram | Operation latency | `operation` |
| `openrustclaw_memory_search_results` | Histogram | Results per search | `query_type` |
| `openrustclaw_core_memory_entries` | Gauge | Core memory size | - |
| `openrustclaw_recall_memory_entries` | Gauge | Recall memory size | - |
| `openrustclaw_memory_maintenance_total` | Counter | Memory maintenance actions | `operation`, `status` |
| `openrustclaw_memory_maintenance_affected_entries` | Histogram | Entries touched by maintenance | `operation`, `status` |
| `openrustclaw_memory_maintenance_duration_seconds` | Histogram | Maintenance latency | `operation` |

The shipped Rust-owned maintenance path now emits these metrics from real archive-summary persistence, archive-source deletion, and old-memory fetches used by the maintenance flow.

### Cache Metrics

| Metric | Type | Description | Labels |
|--------|------|-------------|--------|
| `openrustclaw_cache_operations_total` | Counter | Cache hits/misses | `cache`, `operation` |
| `openrustclaw_cache_size` | Gauge | Entries in cache | `cache` |
| `openrustclaw_cache_evictions_total` | Counter | Cache evictions | `cache`, `reason` |

### Database Metrics

| Metric | Type | Description | Labels |
|--------|------|-------------|--------|
| `openrustclaw_db_queries_total` | Counter | Database queries | `query_type`, `table`, `status` |
| `openrustclaw_db_query_duration_seconds` | Histogram | Query latency | `query_type`, `table` |
| `openrustclaw_db_pool_connections` | Gauge | Connection pool size | `state` |
| `openrustclaw_db_transactions_total` | Counter | Transactions | `operation` |

### Scheduler Metrics

| Metric | Type | Description | Labels |
|--------|------|-------------|--------|
| `openrustclaw_job_executions_total` | Counter | Job executions | `job`, `status` |
| `openrustclaw_job_duration_seconds` | Histogram | Job execution time | `job` |
| `openrustclaw_scheduled_jobs` | Gauge | Total scheduled jobs | - |
| `openrustclaw_jobs_by_state` | Gauge | Jobs per state | `state` |
| `openrustclaw_job_retries_total` | Counter | Job retries | `job`, `retry_count` |

### Security Metrics

| Metric | Type | Description | Labels |
|--------|------|-------------|--------|
| `openrustclaw_auth_attempts_total` | Counter | Auth attempts | `method`, `status` |
| `openrustclaw_origin_checks_total` | Counter | Origin validations | `status` |

The shipped gateway now emits these security metrics from real runtime decisions:

- WebSocket bearer-auth success/failure
- Internal API token success/failure/disabled checks
- WebSocket origin allow/deny decisions
- Webhook rate-limit hits

## Using Metrics in Code

### Recording Metrics

```rust
use openrustclaw_observability::metrics::*;

// Record a request
record_request("GET", "/health", "200");

// Record with timing
let start = std::time::Instant::now();
// ... do work ...
record_request_duration("POST", "/chat", start.elapsed().as_secs_f64());

// Record token usage
record_token_usage("anthropic", "claude-sonnet", 1000, 500);

// Use SimpleTimer for convenience
let timer = SimpleTimer::new();
// ... do work ...
record_provider_duration("openai", "gpt-4", timer.elapsed_secs());
```

### Using Timing Helpers

```rust
use openrustclaw_observability::metrics::SimpleTimer;

async fn handle_chat(request: ChatRequest) -> Result<ChatResponse> {
    let timer = SimpleTimer::new();
    
    let response = process_chat(request).await?;
    
    record_request_duration("POST", "/chat", timer.elapsed_secs());
    Ok(response)
}
```

## Health Checks

The gateway exposes three health check endpoints:

### Basic Health (`/health`)

Lightweight liveness check for load balancers.

```bash
curl http://localhost:8080/health
```

```json
{
  "status": "healthy",
  "service": "openrustclaw-gateway",
  "version": "0.1.0",
  "timestamp": "2024-01-15T10:30:00Z",
  "uptime_seconds": 3600
}
```

### Readiness (`/health/ready`)

Checks that required dependencies are available.

```bash
curl http://localhost:8080/health/ready
```

```json
{
  "status": "ready",
  "checks": {
    "database": { "status": "healthy", "latency_ms": 5 },
    "providers": { "status": "healthy", "available": 3 }
  },
  "timestamp": "2024-01-15T10:30:00Z"
}
```

### Deep Health (`/health/deep`)

Comprehensive check of all subsystems.

```bash
curl http://localhost:8080/health/deep
```

```json
{
  "status": "healthy",
  "components": {
    "database": { "status": "healthy", "latency_ms": 5 },
    "memory": { "status": "healthy" },
    "providers": {
      "status": "healthy",
      "details": {
        "anthropic": "available",
        "openai": "available"
      }
    },
    "scheduler": { "status": "healthy" }
  },
  "timestamp": "2024-01-15T10:30:00Z",
  "duration_ms": 25
}
```

### Configuring Health Checks

```rust
use openrustclaw_gateway::{
    HealthCheckRegistry, HealthState, health_routes,
};
use std::sync::Arc;

// Create health check registry
let mut registry = HealthCheckRegistry::new();

// Register custom health checks
registry.register_fn("custom_check", || async {
    // Perform custom health check
    ComponentHealth::healthy()
});

// Create health state
let health_state = HealthState::new(Arc::new(registry))
    .with_db_pool(db_pool)
    .with_provider_health(|| {
        // Return provider health status
        let mut health = HashMap::new();
        health.insert("anthropic".to_string(), "available".to_string());
        health
    });

// Add routes to your router
let app = Router::new()
    .merge(health_routes())
    .with_state(health_state);
```

## Structured Logging

### Log Levels

| Level | Usage |
|-------|-------|
| `ERROR` | Unrecoverable errors requiring immediate attention |
| `WARN` | Degraded functionality or potential issues |
| `INFO` | Important operational events |
| `DEBUG` | Detailed information for debugging |
| `TRACE` | Very detailed execution tracing |

### Request Tracing

```rust
use openrustclaw_observability::tracing_config::{
    request_span, request_span_with_id, RequestId,
};
use tracing::{info, instrument};

// Automatic request tracing
#[instrument(skip(request))]
async fn handle_request(request: Request) -> Response {
    info!(user_id = %request.user_id, "Processing request");
    // ...
}

// Manual request span with generated ID
let span = request_span("chat_completion");
let _enter = span.enter();

// Or with existing request ID
let request_id = RequestId::new("existing-id");
let span = request_span_with_id("chat_completion", request_id);
```

### Custom Trace Context

```rust
use openrustclaw_observability::tracing_config::{
    TraceContext, extract_trace_context_from_headers,
    inject_trace_context_into_headers,
};

// Extract from incoming request
let ctx = extract_trace_context_from_headers(&request_headers);

// Propagate to outgoing request
let mut outgoing_headers = HeaderMap::new();
inject_trace_context_into_headers(&ctx, &mut outgoing_headers);
```

## Grafana Dashboard

The included dashboard (`grafana/dashboard.json`) provides:

### Overview Panel
- Request Rate (RPS)
- Active Connections
- Error Rate
- Token Rate
- Estimated Cost
- Scheduled Jobs

### Request Metrics
- Request rate by endpoint
- Latency percentiles (p50, p95, p99)
- HTTP error rates by status
- Provider errors by type

### LLM Metrics
- Token usage rate
- Provider latency
- Cost breakdown by provider/model
- Request rate by provider

### Session Metrics
- Active WebSocket connections
- WebSocket message rates
- Memory entry counts
- Cache hit rates

### System Metrics
- Database query latency
- Job execution duration
- Provider availability status
- Jobs by state

## Alerting Rules

Example Prometheus alerting rules:

```yaml
groups:
  - name: openrustclaw
    rules:
      - alert: HighErrorRate
        expr: |
          sum(rate(openrustclaw_requests_total{status=~"5.."}[5m])) 
          / sum(rate(openrustclaw_requests_total[5m])) > 0.05
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High error rate detected"
          
      - alert: ProviderDown
        expr: |
          sum by (provider) (rate(openrustclaw_provider_requests_total[5m])) == 0
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "Provider {{ $labels.provider }} appears down"
          
      - alert: HighLatency
        expr: |
          histogram_quantile(0.99, 
            sum by (le) (rate(openrustclaw_request_duration_seconds_bucket[5m]))
          ) > 5
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "p99 latency exceeds 5 seconds"
          
      - alert: DatabaseSlow
        expr: |
          histogram_quantile(0.95,
            sum by (le) (rate(openrustclaw_db_query_duration_seconds_bucket[5m]))
          ) > 0.5
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Database p95 latency exceeds 500ms"
```

## Best Practices

### Metric Cardinality

- Keep label values bounded (avoid user IDs, timestamps)
- Use status code ranges instead of exact codes where possible
- Group endpoints by pattern rather than full path with IDs

```rust
// Good - bounded cardinality
record_request("GET", "/users/{id}", "200");

// Avoid - unbounded cardinality
record_request("GET", &format!("/users/{}", user_id), "200");
```

### Sampling High-Frequency Events

For very high-frequency events, consider sampling:

```rust
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn record_sampled() {
    // Sample 1% of events
    if COUNTER.fetch_add(1, Ordering::Relaxed) % 100 == 0 {
        record_some_metric();
    }
}
```

### Sensitive Data

Never include sensitive data in metrics or logs:

```rust
// Good
record_request("POST", "/auth", "200");
info!(user_id = %user.id, "User authenticated");

// Avoid
record_request("POST", &format!("/auth/{}", password), "200");
info!(token = %jwt_token, "User authenticated");
```

## Troubleshooting

### Metrics Not Appearing

1. Check that `init_metrics()` was called
2. Verify the `/metrics` endpoint is accessible
3. Confirm Prometheus scrape configuration
4. Check for network/firewall issues

### High Memory Usage

If the metrics exporter uses too much memory:

```rust
// Set idle timeout to remove stale metrics
use openrustclaw_gateway::metrics_endpoint::install_metrics_with_config;

let handle = install_metrics_with_config(Some(Duration::from_secs(300)));
```

### Missing Logs

1. Check `RUST_LOG` environment variable
2. Verify tracing initialization
3. Check log aggregation configuration
4. Ensure log level is not too restrictive

## Integration Examples

### Docker Compose

```yaml
version: '3.8'

services:
  gateway:
    build: .
    environment:
      - RUST_ENV=production
      - RUST_LOG=info
    ports:
      - "8080:8080"

  prometheus:
    image: prom/prometheus
    volumes:
      - ./prometheus/prometheus.yml:/etc/prometheus/prometheus.yml
    ports:
      - "9090:9090"

  grafana:
    image: grafana/grafana
    volumes:
      - ./grafana/dashboard.json:/var/lib/grafana/dashboards/openrustclaw.json
    ports:
      - "3000:3000"
```

### Kubernetes

```yaml
apiVersion: v1
kind: Service
metadata:
  name: openrustclaw-gateway
  annotations:
    prometheus.io/scrape: "true"
    prometheus.io/port: "8080"
    prometheus.io/path: "/metrics"
spec:
  ports:
    - port: 8080
      targetPort: 8080
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: openrustclaw-gateway
spec:
  template:
    spec:
      containers:
        - name: gateway
          image: openrustclaw/gateway:latest
          ports:
            - containerPort: 8080
          livenessProbe:
            httpGet:
              path: /health
              port: 8080
            initialDelaySeconds: 10
            periodSeconds: 10
          readinessProbe:
            httpGet:
              path: /health/ready
              port: 8080
            initialDelaySeconds: 5
            periodSeconds: 5
```
