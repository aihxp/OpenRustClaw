# Testing Patterns

**Analysis Date:** 2026-03-26

## Overall Strategy

Testing is layered rather than centralized in one harness:

- crate-local unit tests inside source files (`mod tests`)
- crate-specific integration suites for some provider/integration crates
- workspace-level integration coverage in `tests/integration/`
- workspace-level end-to-end coverage in `tests/e2e/`
- Python sidecar tests in `sidecar/test_sidecar.py`

CI currently enforces:
- `cargo check --workspace`
- `cargo test --workspace --lib`
- `cargo clippy --workspace -- -D warnings`
- `cargo fmt --all -- --check`
- runtime budget checks via `scripts/check-runtime-budgets.sh`

## Main Test Locations

**Workspace-level:**
- `tests/integration/`
- `tests/e2e/`

**Representative crate-local integration suites:**
- `crates/anthropic_rust/tests/integration_tests.rs`
- `crates/cursor/tests/integration_tests.rs`
- `crates/distributed/tests/integration_tests.rs`
- `crates/automation/tests/integration_tests.rs`
- `crates/mcp2cli/tests/integration_tests.rs`
- `crates/ai21/tests/integration_tests.rs`
- `crates/cohere/tests/integration_tests.rs`

**Python sidecar:**
- `sidecar/test_sidecar.py`

## Common Test Patterns

**Async Rust tests:**
- `#[tokio::test]` is widely used for runtime/provider/channel behavior
- `serial_test` appears in workspace test dependencies for flows that need isolation

**Inline unit tests:**
- Many modules define `mod tests` directly in the source file
- This is common in `crates/security/`, `crates/memory/`, `crates/gateway/`, `crates/skills/`, provider SDK crates, and command modules

**Integration-style behavioral tests:**
- The workspace `tests/integration/src/` suite targets:
  - agent runtime
  - gateway
  - memory workflow
  - provider chain behavior
  - scheduler behavior
  - MCP/tool schema translations
  - documented scenarios and fixture suites

**E2E coverage:**
- `tests/e2e/src/` covers provider fallback, chat workflow, memory workflow, scheduler workflow, and security workflow

## Coverage Strengths

- Broad crate coverage across providers, channels, security, memory, gateway, and runtime flows
- Dedicated integration and E2E crates exist instead of relying only on unit tests
- CI enforces compile, formatting, lints, and budget checks, not just test execution

## Coverage Gaps / Caveats

- CI only runs `cargo test --workspace --lib`, so some non-lib or example-driven paths are not exercised in the main CI lane
- The Python sidecar test suite is separate from the Rust CI path shown in `.github/workflows/ci.yml`
- Very large command files such as `crates/cli/src/commands/start.rs` and `skills.rs` increase the risk of logic clusters that are hard to cover comprehensively
- The repo has many integration surfaces and optional features; not all external systems are realistically exercised in default CI

## Safe Change Guidance

- When touching shared traits or command/control surfaces, look for both inline tests and workspace-level integration coverage
- For provider/channel changes, check whether there is an existing crate-local integration suite before adding a new harness
- For sidecar-related changes, validate both Rust-side contracts and `sidecar/test_sidecar.py`
- For operator/runtime changes, inspect `.github/workflows/ci.yml` to see what is actually enforced on PRs

## High-Signal Paths

- `.github/workflows/ci.yml`
- `tests/integration/src/`
- `tests/e2e/src/`
- `sidecar/test_sidecar.py`
- `crates/cli/src/commands/start.rs`

---
*Testing analysis: 2026-03-26*
*Update when CI gates or major test harnesses change*
