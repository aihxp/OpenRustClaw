# Coding Conventions

**Analysis Date:** 2026-04-04

## Naming Patterns

**Files:**
- Use `snake_case` for Rust modules and Python modules: `crates/security/src/origin_check.rs`, `crates/gateway/src/metrics_endpoint.rs`, `sidecar/src/workflow_contract.py`.
- Use descriptive `_test.rs` filenames for multi-crate Rust integration coverage: `tests/integration/src/provider_chain_test.rs`, `tests/integration/src/security_posture_test.rs`.
- Use `test_*.rs` and category folders for E2E suites: `tests/e2e/src/test_provider_fallback.rs`, `tests/e2e/tests/smoke/test_health.rs`, `tests/e2e/tests/regression/test_memory_recall.rs`.
- Use `test_*.py` and `test_*`/`async def test_*` functions for Python sidecar tests: `sidecar/test_sidecar.py`.

**Functions:**
- Use `snake_case` for Rust and Python functions: `parse_workflow_request` in `sidecar/src/workflow_contract.py`, `install_metrics_with_config` in `crates/gateway/src/metrics_endpoint.rs`.
- Use `new` for constructors and `with_*` for configuration builders: `OriginValidator::new` in `crates/security/src/origin_check.rs`, `ProviderChain::with_cooldown` in `tests/integration/src/provider_chain_test.rs`, `MockRateLimitedProvider::with_retry_after` in `tests/e2e/src/common/mod.rs`.
- Name tests as behavior statements instead of generic cases: `accepts_allowed_origin` in `crates/security/src/origin_check.rs`, `falls_back_on_rate_limit` in `tests/integration/src/provider_chain_test.rs`, `test_rag_pipeline_supports_required_source_filters` in `sidecar/test_sidecar.py`.

**Variables:**
- Use `snake_case` locals and fields consistently: `allowed_origins` in `crates/security/src/origin_check.rs`, `current_step` in `sidecar/src/server.py`, `mock_server` in `crates/anthropic_rust/tests/integration_tests.rs`.
- Prefer explicit domain names over short abbreviations in public code: `origin_validator` in `tests/e2e/src/common/mod.rs`, `workflow_builder` and `parsed_request` in `sidecar/src/server.py`.

**Types:**
- Use `UpperCamelCase` for structs, enums, traits, and error types: `OriginValidator` in `crates/security/src/origin_check.rs`, `MetricsState` in `crates/gateway/src/metrics_endpoint.rs`, `WorkflowContractError` in `sidecar/src/workflow_contract.py`.
- Keep enums semantically named and variant-rich rather than stringly typed: `ProviderError`, `SecurityError`, and `Error` in `crates/core/src/error.rs`.

## Code Style

**Formatting:**
- Rust formatting is driven by the default `rustfmt` toolchain. No repo-level `rustfmt.toml` or `.rustfmt.toml` was detected at the repo root.
- Contributor docs in `docs/src/contributing/development.md` standardize on `cargo fmt --all` and `cargo fmt --all -- --check`.
- Python formatting is standardized in `sidecar/pyproject.toml`.
- Key Python settings from `sidecar/pyproject.toml`:
  - `line-length = 100`
  - `target-version = "py311"`

**Linting:**
- Rust linting is standardized through Clippy commands documented in `docs/src/contributing/development.md`: `cargo clippy --workspace --all-targets --all-features`.
- No repo-level `clippy.toml` was detected at the repo root.
- Python linting is enforced through Ruff in `sidecar/pyproject.toml`.
- Key Ruff settings in `sidecar/pyproject.toml`:
  - `select = ["E", "F", "W", "I", "N", "D"]`
  - `ignore = ["D100", "D104"]`
- Python typing is part of the convention. `sidecar/pyproject.toml` sets `disallow_untyped_defs = true`, so new sidecar functions should be explicitly typed.

## Import Organization

**Order:**
1. Standard library imports first: `std::sync::Arc` in `crates/security/src/origin_check.rs`, `json` and `threading` in `sidecar/src/server.py`.
2. Third-party crates and packages second: `axum`, `metrics_exporter_prometheus`, `tokio`, `wiremock`, `grpc`, `pytest`.
3. Internal workspace crates or local relative modules last: `openrustclaw_core::*` in `crates/gateway/src/metrics_endpoint.rs`, `.proto` and `.workflows.*` imports in `sidecar/src/server.py`.

**Path Aliases:**
- Rust uses crate names directly instead of alias layers: `openrustclaw_core`, `openrustclaw_gateway`, `openrustclaw_memory`, `anthropic_rust`.
- Python sidecar uses normal relative imports from `sidecar/src/`: `.proto`, `.langsmith_bridge`, `.workflows.agent_orchestrator`.
- No custom Rust or Python path alias system was detected in `Cargo.toml` or `sidecar/pyproject.toml`.

## Error Handling

**Patterns:**
- Library crates use `thiserror`-based domain errors. The canonical pattern is centralized in `crates/core/src/error.rs`.
- Return the workspace `Result<T>` alias from library-facing logic and convert underlying errors into specific variants such as `ProviderError`, `DatabaseError`, or `SecurityError`.
- Use `anyhow` at CLI/application boundaries. `crates/cli/src/commands/mcp2cli.rs` imports `anyhow::{Context, Result}` and adds user-facing context with `Context`/`anyhow!`/`bail!`.
- Prefer structured early returns over panics in runtime code. For example, `sidecar/src/server.py` catches `WorkflowContractError` and returns a structured `WorkflowResponse` with `status="error"`.
- Use `expect`/`unwrap` freely in tests when failure should abort the test immediately: `tests/integration/src/common.rs`, `tests/e2e/src/common/mod.rs`, `crates/anthropic_rust/tests/integration_tests.rs`.

## Logging

**Framework:** `tracing` in Rust, standard `logging` in Python

**Patterns:**
- Use structured `tracing` macros in Rust runtime code: `warn!(origin = %origin, "...")` in `crates/security/src/origin_check.rs`, `info!(command_id = %slash_command.command_id, "...")` in `crates/channels/src/google_chat.rs`.
- Prefer structured fields over interpolated strings when the fields matter operationally.
- Use `#[instrument(...)]` on request handlers or cross-cutting async boundaries where tracing context matters, as in `metrics_handler` in `crates/gateway/src/metrics_endpoint.rs`.
- Keep `println!` scoped to CLI commands, examples, and test runners. Runtime/library crates use `tracing`; visible `println!` usage is concentrated in `crates/cli/src/commands/*.rs`, `tests/e2e/src/main.rs`, and example files.
- Python sidecar logging uses a module logger: `logger = logging.getLogger(__name__)` in `sidecar/src/server.py`, with `logger.info`, `logger.error`, and `logger.exception` for workflow lifecycle reporting.

## Comments

**When to Comment:**
- Add module-level docs with `//!` in Rust and triple-quoted module docstrings in Python. Representative files: `crates/security/src/origin_check.rs`, `crates/gateway/src/metrics_endpoint.rs`, `sidecar/src/server.py`.
- Use `///` on public Rust items that form the external API or clarify a non-obvious responsibility.
- Use short inline comments for important transitions, setup steps, or test section boundaries. Examples: migration comments in `tests/integration/src/common.rs`, section dividers in `tests/e2e/src/common/mod.rs`, and workflow comments in `sidecar/src/server.py`.
- Avoid redundant line-by-line narration. Existing comments tend to explain intent, not syntax.

**JSDoc/TSDoc:**
- Not applicable. The repo does not use TypeScript.

## Function Design

**Size:** Prefer medium-sized functions with explicit setup over dense helper abstractions
- Many core functions do one end-to-end task with visible steps: `parse_workflow_request` in `sidecar/src/workflow_contract.py`, `install_metrics` in `crates/gateway/src/metrics_endpoint.rs`.
- Large files exist, but even inside them the code is split into focused helpers and inline test modules: `crates/channels/src/google_chat.rs`, `crates/cli/src/commands/mcp2cli.rs`.

**Parameters:** Prefer explicit structs and named arguments over positional tuples
- Rust request flows pass typed structs like `CompletionRequest` or `WorkflowResponse` instead of loose maps: `tests/integration/src/provider_chain_test.rs`, `sidecar/src/server.py`.
- Builder-style APIs are used when requests are complex: `MessageRequest::builder(...)` in `crates/anthropic_rust/tests/integration_tests.rs`.

**Return Values:** Prefer typed `Result<T>` or explicit domain objects
- Rust library code returns `Result<T>` from `openrustclaw_core::error`.
- Python helpers return dataclasses or dictionaries with clear shape: `ParsedWorkflowRequest` in `sidecar/src/workflow_contract.py`, `WorkflowRegistry.get()` in `sidecar/src/server.py`.

## Module Design

**Exports:** Re-export public APIs through `lib.rs` when a crate wants a stable top-level surface
- `crates/openrouter_api/src/lib.rs` and `crates/observability/src/lib.rs` use `pub use` to present curated crate APIs.
- Test crates also re-export shared helpers: `tests/e2e/src/lib.rs` re-exports `common::*`.

**Barrel Files:** Used selectively
- Rust crate roots use `lib.rs` as the barrel layer.
- Test crates use `common.rs` or `common/mod.rs` as shared helper entry points: `tests/integration/src/common.rs`, `tests/e2e/src/common/mod.rs`.
- Python sidecar does not use barrel modules heavily; it imports directly from concrete files under `sidecar/src/`.

## Prescriptive Guidance

- Follow the established import grouping: stdlib, third-party, internal crate or local imports.
- Keep new Rust modules and new test files in `snake_case`.
- Add module docs for new Rust modules that define a meaningful subsystem or external integration.
- Use `thiserror`-based domain errors in library crates and reserve `anyhow` for CLI/app entry points.
- Use `tracing` macros for runtime observability; only use `println!` in CLI UX, examples, and tests.
- For sidecar Python, keep full type annotations and stay within the Ruff/Mypy contract from `sidecar/pyproject.toml`.
- Reuse `common` test helper modules instead of duplicating provider mocks, tempdir setup, or tracing initialization.

---

*Convention analysis: 2026-04-04*
