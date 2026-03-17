# OpenRustClaw E2E Tests

Comprehensive end-to-end testing framework for OpenRustClaw that verifies system-wide functionality including chat workflows, memory management, scheduling, security, and provider resilience.

## Overview

This E2E test suite provides:

- **10+ complete test scenarios** across 5 major workflows
- **Mock-based testing** for fast, deterministic CI runs
- **Live provider testing** for integration validation
- **HTTP mocking** with Wiremock for external dependencies
- **Test isolation** with `serial_test`

## Test Structure

```
tests/e2e/
├── Cargo.toml                  # Test dependencies
├── README.md                   # This file
└── src/
    ├── lib.rs                  # Test entry point
    ├── main.rs                 # CLI runner binary
    ├── common/
    │   └── mod.rs              # Shared test utilities
    ├── test_chat_workflow.rs   # Chat & conversation tests
    ├── test_memory_workflow.rs # Memory storage tests
    ├── test_scheduler_workflow.rs # Job scheduler tests
    ├── test_security_workflow.rs  # Security & auth tests
    └── test_provider_fallback.rs  # Provider resilience tests
```

## Running Tests

### Basic Usage

```bash
# Run all E2E tests (mock mode)
cargo test --test e2e

# Run with output visible
cargo test --test e2e -- --nocapture

# Run specific test file
cargo test --test e2e test_chat_workflow

# Run a specific test
cargo test --test e2e test_simple_chat_message
```

### Live Provider Testing

To run tests against real LLM providers:

```bash
# Enable live mode
export E2E_LIVE=1

# Configure providers (at least one required)
export OPENAI_API_KEY=sk-...
export ANTHROPIC_API_KEY=sk-ant-...
export OLLAMA_HOST=http://localhost:11434

# Run tests
cargo test --test e2e
```

### Binary Runner

```bash
# Show test information
cargo run --bin e2e

# With live mode
cargo run --bin e2e -- E2E_LIVE=1
```

## Test Scenarios

### Chat Workflow Tests (`test_chat_workflow.rs`)

| Test | Description |
|------|-------------|
| `test_simple_chat_message` | Basic user message gets response |
| `test_chat_with_tool_call` | Agent uses tools to answer |
| `test_multi_turn_conversation` | Context maintained across turns |
| `test_conversation_stored_in_memory` | Chat history persisted |
| `test_core_memory_update` | User preferences stored |
| `test_streaming_response` | Token streaming works |
| `test_session_isolation` | Sessions don't leak data |

### Memory Workflow Tests (`test_memory_workflow.rs`)

| Test | Description |
|------|-------------|
| `test_store_and_retrieve_semantic_memory` | Store and search knowledge |
| `test_store_episodic_memory_with_session` | Conversation context stored |
| `test_search_memories_across_sessions` | Cross-session search |
| `test_memory_deduplication` | Duplicate detection by hash |
| `test_memory_expiration` | TTL-based cleanup |
| `test_core_memory_operations` | Key-value core memory |
| `test_update_core_memory` | Modify existing entries |
| `test_memory_search_confidence_threshold` | Filter by confidence |
| `test_context_manager_integration` | Core + recall integration |
| `test_memory_export_import` | Backup and restore |

### Scheduler Workflow Tests (`test_scheduler_workflow.rs`)

| Test | Description |
|------|-------------|
| `test_create_one_time_job` | Schedule immediate job |
| `test_create_interval_job` | Recurring schedule |
| `test_job_execution_tracking` | Run status updates |
| `test_job_retry_on_failure` | Automatic retries |
| `test_dead_letter_queue` | Failed job handling |
| `test_job_lease_prevents_duplicate_execution` | Worker coordination |
| `test_job_state_transitions` | Pause/resume jobs |
| `test_job_due_check` | Schedule evaluation |
| `test_job_idempotency_key` | Duplicate prevention |
| `test_job_run_status_lifecycle` | Status tracking |

### Security Workflow Tests (`test_security_workflow.rs`)

| Test | Description |
|------|-------------|
| `test_websocket_requires_auth` | Auth enforcement |
| `test_websocket_accepts_valid_auth` | Valid token accepted |
| `test_origin_validation_rejects_invalid` | CORS protection |
| `test_origin_validation_wildcard` | Allow any origin mode |
| `test_input_sanitization_xss` | XSS prevention |
| `test_rate_limiting` | Request throttling |
| `test_audit_event_logging` | Security events |
| `test_session_isolation` | Data boundaries |
| `test_gateway_header_validation` | Required headers |
| `test_security_headers` | HTTP security |
| `test_sql_injection_prevention` | SQL injection protection |
| `test_path_traversal_prevention` | Path security |

### Provider Fallback Tests (`test_provider_fallback.rs`)

| Test | Description |
|------|-------------|
| `test_primary_provider_succeeds` | Normal operation |
| `test_fallback_to_secondary` | Failover on error |
| `test_fallback_on_rate_limit` | Handle 429 responses |
| `test_all_providers_fail` | Complete outage handling |
| `test_provider_cooldown_respected` | Rate limit backoff |
| `test_multiple_fallbacks` | Chain of providers |
| `test_provider_chain_info` | Chain metadata |
| `test_manual_cooldown_clear` | Reset rate limits |
| `test_empty_provider_chain` | No providers configured |
| `test_token_usage_through_fallback` | Cost tracking |
| `test_live_provider_chain` | Real provider check |

## Test Utilities

### Mock Providers

- `MockSuccessProvider` - Returns configured response
- `MockRateLimitedProvider` - Simulates 429 errors
- `MockUnavailableProvider` - Simulates 5xx errors
- `MockConversationalProvider` - Multi-turn responses
- `MockToolCallingProvider` - Tool invocation flow

### Mock Tools

- `EchoTool` - Echoes input
- `CalculatorTool` - Basic arithmetic
- `FailingTool` - Always fails
- `MemoryStoreTool` - Stores to recall memory

### Test Environment

```rust
let env = TestEnvironment::new().await;

// Access components
env.db_pool           // SQLite connection pool
env.context_manager   // Memory context
env.recall_memory     // Recall memory store
env.core_memory       // Core memory store
env.session_manager   // Session management

// Helper methods
env.start_gateway(require_auth).await  // Start test server
env.store_memory(entry).await          // Store memory
env.search_memories(query).await       // Search memories
```

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `E2E_LIVE` | Enable live provider tests | `0` |
| `OPENAI_API_KEY` | OpenAI API key | - |
| `ANTHROPIC_API_KEY` | Anthropic API key | - |
| `OLLAMA_HOST` | Ollama endpoint | - |
| `RUST_LOG` | Log level | `info` |

## CI Integration

```yaml
# GitHub Actions example
- name: Run E2E tests
  run: cargo test --test e2e
  env:
    RUST_LOG: warn
```

## Adding New Tests

1. Add test file to `src/test_<name>.rs`
2. Import from `lib.rs`
3. Use `#[tokio::test]` and `#[serial]`
4. Initialize tracing: `init_test_tracing()`
5. Create test environment: `TestEnvironment::new().await`

Example:

```rust
#[tokio::test]
#[serial]
async fn test_my_feature() {
    init_test_tracing();
    let env = TestEnvironment::new().await;
    
    // Your test code here
    assert!(true);
}
```

## Troubleshooting

### Tests fail with port already in use
Tests use random ports, but if you see conflicts:
```bash
# Kill processes on test ports
lsof -ti:3000-3100 | xargs kill -9
```

### Database locked errors
SQLite concurrency issue - tests run serially via `#[serial]`

### Live tests timeout
Check API key validity and network connectivity:
```bash
curl https://api.openai.com/v1/models \
  -H "Authorization: Bearer $OPENAI_API_KEY"
```
