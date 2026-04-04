# Testing Patterns

**Analysis Date:** 2026-04-04

## Test Framework

**Runner:**
- Rust: built-in `cargo test` across the workspace declared in `Cargo.toml`
- Rust async tests: `tokio` test runtime via `#[tokio::test]`
- Python sidecar: `pytest` with `pytest-asyncio`
- Config: `sidecar/pyproject.toml` for Python; no standalone Rust test config file was detected

**Assertion Library:**
- Rust standard assertions: `assert_eq!`, `assert!`, `matches!`, `unwrap_err`, `expect`
- Python built-in `assert` plus `pytest.skip`

**Run Commands:**
```bash
cargo test --workspace                              # Run all Rust tests
cargo test --workspace --lib                        # Run inline crate unit tests
cargo test -p openrustclaw-integration-tests --quiet  # Run the multi-crate integration crate
cargo test -p openrustclaw-e2e-tests --quiet       # Run the E2E crate
pytest sidecar/test_sidecar.py                     # Run sidecar tests under pytest
python sidecar/test_sidecar.py                     # Run the script-style sidecar test runner
```

## Test File Organization

**Location:**
- Unit tests are primarily inline with production code under `crates/*/src/` via `#[cfg(test)] mod tests`.
- Multi-crate integration tests live in the dedicated crate `tests/integration/src/`.
- Slower workflow and system tests live in the dedicated crate `tests/e2e/`.
- Python sidecar tests currently live in a single top-level file: `sidecar/test_sidecar.py`.

**Naming:**
- Inline Rust unit tests use behavior names inside `mod tests`: `accepts_allowed_origin` in `crates/security/src/origin_check.rs`.
- Rust integration and system files use descriptive `_test.rs` or `test_*.rs` names: `tests/integration/src/provider_chain_test.rs`, `tests/e2e/src/test_security_workflow.rs`.
- E2E category suites are grouped by concern under `tests/e2e/tests/`: `smoke/`, `vertical/`, `horizontal/`, `regression/`.
- Python uses `test_*` and `async def test_*` in `sidecar/test_sidecar.py`.

**Structure:**
```text
OpenRustClaw/
├── crates/
│   └── */src/
│       └── ... #[cfg(test)] mod tests ...
├── tests/
│   ├── integration/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── common.rs
│   │       ├── provider_chain_test.rs
│   │       └── ...
│   └── e2e/
│       ├── Cargo.toml
│       ├── src/
│       │   ├── common/
│       │   ├── test_chat_workflow.rs
│       │   └── ...
│       └── tests/
│           ├── smoke/
│           ├── vertical/
│           ├── horizontal/
│           └── regression/
└── sidecar/
    └── test_sidecar.py
```

## Test Structure

**Suite Organization:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_allowed_origin() {
        let validator = OriginValidator::new(vec!["https://example.com".to_string()]);
        assert!(validator.validate("https://example.com").is_ok());
    }
}
```
- Pattern source: `crates/security/src/origin_check.rs`

```rust
#[tokio::test]
async fn falls_back_on_rate_limit() {
    init_test_tracing();

    let chain = ProviderChain::new(vec![
        Arc::new(MockRateLimitedProvider::new("primary")),
        Arc::new(MockSuccessProvider::new("fallback", "Hello from fallback")),
    ]);

    let response = chain.complete(make_request()).await.unwrap();
    assert_eq!(response.provider, "fallback");
}
```
- Pattern source: `tests/integration/src/provider_chain_test.rs`

**Patterns:**
- Initialize tracing once for Rust integration/E2E tests with `init_test_tracing()` from `tests/integration/src/common.rs` or `tests/e2e/src/common/mod.rs`.
- Use `#[tokio::test]` for async Rust tests across crates, integration, and E2E layers.
- Use `#[serial]` from `serial_test` when tests share global state, tracing setup, cooldown state, or env-sensitive live-provider behavior. This is common in `tests/e2e/src/test_chat_workflow.rs`, `tests/e2e/src/test_memory_workflow.rs`, `tests/e2e/src/test_provider_fallback.rs`, `tests/e2e/src/test_scheduler_workflow.rs`, and `tests/e2e/src/test_security_workflow.rs`.
- Prefer explicit scenario blocks and literal request structs over macro-heavy helpers.
- For Python, mix ordinary pytest test functions with async pytest tests in the same file because `sidecar/pyproject.toml` sets `asyncio_mode = "auto"`.

## Mocking

**Framework:** `wiremock` plus handwritten test doubles

**Patterns:**
```rust
let mock_server = MockServer::start().await;

Mock::given(method("POST"))
    .and(path("/v1/messages"))
    .respond_with(ResponseTemplate::new(200).set_body_json(&mock_response))
    .mount(&mock_server)
    .await;
```
- Pattern source: `crates/anthropic_rust/tests/integration_tests.rs`

```rust
let chain = ProviderChain::new(vec![
    Arc::new(MockRateLimitedProvider::new("primary")),
    Arc::new(MockSuccessProvider::new("fallback", "Hello")),
]);
```
- Pattern source: `tests/integration/src/provider_chain_test.rs`

**What to Mock:**
- Mock external HTTP APIs and provider endpoints with `wiremock`: `crates/anthropic_rust/tests/integration_tests.rs`, `crates/channels/src/google_chat.rs`, `crates/channels/src/slack.rs`, `crates/mcp2cli/src/adapters/mcp_adapter.rs`.
- Mock provider behavior at trait boundaries with handwritten `LlmProvider` implementations in shared helpers: `tests/integration/src/common.rs`, `tests/e2e/src/common/mod.rs`.
- In Python, skip tests gracefully when optional sidecar dependencies are unavailable rather than inventing local stubs. This pattern is centralized in `_skip_missing_dependency()` in `sidecar/test_sidecar.py`.

**What NOT to Mock:**
- Do not mock core SQLite-backed behavior when testing integration or E2E flows. The existing pattern is to spin up real temporary databases and run migrations through `openrustclaw_db::run_migrations()` in `tests/integration/src/common.rs` and `tests/e2e/src/common/mod.rs`.
- Do not mock the gateway router when validating HTTP/system behavior. `tests/e2e/src/common/mod.rs` starts a real Axum server with `GatewayServer::router(...)`.
- Do not replace memory policies or provider-chain control logic with stubs when the point of the test is the real algorithm. Existing suites use the actual implementations and assert their outputs.

## Fixtures and Factories

**Test Data:**
```rust
pub struct TestUsers;

impl TestUsers {
    pub fn alice() -> TestUser {
        TestUser {
            id: "user-alice-001".to_string(),
            name: "Alice".to_string(),
            email: "alice@example.com".to_string(),
        }
    }
}
```
- Pattern source: `tests/e2e/src/common/fixtures.rs`

```rust
pub async fn create_test_db() -> SqlitePool {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(5)
        .connect("sqlite::memory:")
        .await
        .expect("Failed to create test database pool");

    openrustclaw_db::run_migrations(&pool).await.expect("Failed to run migrations");
    pool
}
```
- Pattern source: `tests/integration/src/common.rs`

**Location:**
- Shared Rust integration fixtures live in `tests/integration/src/common.rs`.
- Shared Rust E2E fixtures and helpers live in `tests/e2e/src/common/mod.rs`, `tests/e2e/src/common/fixtures.rs`, `tests/e2e/src/common/assertions.rs`, and `tests/e2e/src/common/http_client.rs`.
- Python sidecar keeps its helper setup inside `sidecar/test_sidecar.py`; no separate `conftest.py` was detected.

## Coverage

**Requirements:** None enforced by config
- No workspace-level coverage threshold or CI coverage config was detected in `Cargo.toml`, `tests/`, or `sidecar/pyproject.toml`.
- Coverage commands are documented rather than enforced:
  - Rust tarpaulin commands in `docs/src/contributing/development.md`
  - Python `pytest --cov=src --cov-report=html` in `docs/src/contributing/development.md`

**View Coverage:**
```bash
cargo tarpaulin --workspace --out Html
cd sidecar && pytest --cov=src --cov-report=html
```

## Test Types

**Unit Tests:**
- Inline crate-local tests dominate the Rust codebase. `rg -n "#\\[cfg\\(test\\)\\]" crates -g '*.rs'` reports roughly 365 inline test modules.
- Use these for pure logic, parsers, formatters, validators, and crate-local transformations: `crates/security/src/origin_check.rs`, `crates/openrouter_api/src/routing.rs`, `crates/openrouter_api/src/models.rs`, `crates/gateway/src/metrics_endpoint.rs`.

**Integration Tests:**
- The `tests/integration/` crate stitches together multiple workspace crates with real tempdirs, real SQLite, and shared mock providers.
- Representative coverage includes provider chains, gateway behavior, MCP, onboarding, memory policy, runtime/operator reports, and documented scenarios: `tests/integration/src/provider_chain_test.rs`, `tests/integration/src/gateway_test.rs`, `tests/integration/src/mcp_test.rs`, `tests/integration/src/documented_scenario_test.rs`.

**E2E Tests:**
- The `tests/e2e/` crate focuses on workflow behavior and system boundaries.
- It includes both workflow files under `tests/e2e/src/` and categorized suites under `tests/e2e/tests/`.
- Categories are explicit in `tests/e2e/tests/e2e_tests.rs`: `smoke`, `vertical`, `horizontal`, and `regression`.
- Live-provider checks are opt-in via `E2E_LIVE=1`, as documented in `tests/e2e/README.md` and implemented in `tests/e2e/src/common/mod.rs`.

## Common Patterns

**Async Testing:**
```rust
#[tokio::test]
#[serial]
async fn test_provider_cooldown_respected() {
    init_test_tracing();
    let chain = ProviderChain::with_cooldown(...);
    let response = chain.complete(request.clone()).await.expect("Should succeed");
    assert_eq!(response.provider, "secondary");
}
```
- Pattern source: `tests/e2e/src/test_provider_fallback.rs`

```python
async def test_agent_workflow():
    graph = build_agent_graph()
    async for event in graph.astream(initial_state, config=config):
        print(f"  Event: {list(event.keys())}")
```
- Pattern source: `sidecar/test_sidecar.py`

**Error Testing:**
```rust
let result = client.messages().create(request).await;
assert!(result.is_err());
let err = result.unwrap_err();
assert!(err.to_string().contains("Authentication"));
```
- Pattern source: `crates/anthropic_rust/tests/integration_tests.rs`

```python
with pytest.raises(WorkflowContractError):
    parse_workflow_request(request)
```
- Use this style for new Python contract tests, following the exception-based parsing in `sidecar/src/workflow_contract.py` and the existing parsing assertions in `sidecar/test_sidecar.py`.

## Prescriptive Guidance

- Add fast crate-local checks inline with `#[cfg(test)] mod tests` unless the behavior crosses crate or process boundaries.
- Put multi-crate behavior in `tests/integration/src/` and reuse `tests/integration/src/common.rs`.
- Put workflow, server, and system-boundary checks in `tests/e2e/`, and reuse the helpers in `tests/e2e/src/common/`.
- Mock external services, not internal domain behavior.
- Use temporary directories or in-memory SQLite instead of persistent filesystem state.
- Keep live-provider tests explicitly opt-in with `E2E_LIVE=1`; default suites should stay deterministic and offline.
- For sidecar work, keep tests compatible with both `pytest` and `python test_sidecar.py` unless the sidecar test layout is intentionally refactored.

---

*Testing analysis: 2026-04-04*
