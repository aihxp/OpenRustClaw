# OpenRustClaw E2E Testing Framework

This directory contains comprehensive End-to-End (E2E) tests following the **NSEW** methodology:

| Type | Description | Runtime |
|------|-------------|---------|
| **Horizontal E2E** | Full user journeys across all subsystems | ~5-10 min |
| **Vertical E2E** | Deep testing of individual layers | ~3-5 min |
| **Smoke E2E** | Critical path tests only | ~30-60 sec |
| **Regression E2E** | Full comprehensive test suite | ~15-20 min |

## Quick Start

```bash
# Run smoke tests (fast, CI-friendly)
cargo test --test e2e smoke

# Run horizontal tests (user journeys)
cargo test --test e2e horizontal

# Run vertical tests (layer-specific)
cargo test --test e2e vertical

# Run regression tests (full suite)
cargo test --test e2e regression

# Run all E2E tests
cargo test --test e2e
```

## Test Organization

```
tests/e2e/src/
├── lib.rs                    # Test runner entry point
├── main.rs                   # CLI runner
├── common/                   # Shared utilities
│   ├── mod.rs
│   ├── fixtures.rs           # Test data fixtures
│   ├── assertions.rs         # Custom assertions
│   └── http_client.rs        # HTTP test client
├── horizontal/               # Full journey tests
│   ├── mod.rs
│   ├── test_chat_journey.rs
│   ├── test_agent_workflow.rs
│   ├── test_mcp_integration.rs
│   └── test_memory_context.rs
├── vertical/                 # Layer-specific tests
│   ├── mod.rs
│   ├── test_api_layer.rs
│   ├── test_db_layer.rs
│   ├── test_provider_layer.rs
│   └── test_gateway_layer.rs
├── smoke/                    # Critical path tests
│   ├── mod.rs
│   ├── test_health.rs
│   ├── test_core_flows.rs
│   └── test_connectivity.rs
└── regression/               # Comprehensive tests
    ├── mod.rs
    ├── test_all_providers.rs
    ├── test_memory_recall.rs
    ├── test_scheduler_recovery.rs
    └── test_security_boundaries.rs
```

## Environment Variables

| Variable | Description |
|----------|-------------|
| `E2E_LIVE` | Enable live provider tests (requires API keys) |
| `E2E_SMOKE_ONLY` | Run only smoke tests |
| `E2E_PARALLEL` | Run tests in parallel |
| `E2E_TIMEOUT_SECS` | Test timeout (default: 300) |
| `OPENAI_API_KEY` | OpenAI API key for live tests |
| `ANTHROPIC_API_KEY` | Anthropic API key for live tests |

## Test Tags

Tests are tagged for selective execution:

- `#[smoke]` - Critical path, runs on every commit
- `#[horizontal]` - Full user journeys
- `#[vertical]` - Layer-specific tests
- `#[regression]` - Comprehensive coverage
- `#[live]` - Requires live API keys
- `#[slow]` - Long-running tests
- `#[flaky]` - Known flaky tests (retry enabled)

## Architecture

### Horizontal Tests

Simulate real user workflows end-to-end:
- User sends message → Gateway → Agent → LLM → Tool execution → Response
- Agent session with memory → Context retrieval → LLM completion
- MCP tool discovery → Tool execution → Result storage

### Vertical Tests

Test each layer in isolation:
- **API Layer**: HTTP endpoints, WebSocket, SSE
- **DB Layer**: SQLite operations, migrations, queries
- **Provider Layer**: LLM provider fallbacks, retries
- **Gateway Layer**: Rate limiting, auth, routing

### Smoke Tests

Fast sanity checks:
- Health endpoints respond
- Database connections work
- Core flows execute
- Critical dependencies available

### Regression Tests

Comprehensive edge cases:
- All provider combinations
- Memory eviction under pressure
- Scheduler failure recovery
- Security boundary enforcement
