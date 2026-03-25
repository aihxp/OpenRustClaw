# Testing Guide

OpenRustClaw keeps three test layers in active use:

- inline unit tests inside the owning crate modules
- multi-crate integration tests in `tests/integration/`
- longer workflow and end-to-end tests in `tests/e2e/`

## Current Test Layout

```text
OpenRustClaw/
├── crates/
│   └── */src/
│       └── ... inline unit tests ...
├── tests/
│   ├── integration/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── agent_runtime_test.rs
│   │       ├── fixture_suite_test.rs
│   │       ├── gateway_test.rs
│   │       ├── mcp_test.rs
│   │       ├── memory_workflow_test.rs
│   │       ├── provider_chain_test.rs
│   │       ├── scheduler_test.rs
│   │       ├── security_test.rs
│   │       └── common.rs
│   └── e2e/
│       ├── Cargo.toml
│       └── src/
│           ├── test_chat_workflow.rs
│           ├── test_memory_workflow.rs
│           ├── test_provider_fallback.rs
│           ├── test_scheduler_workflow.rs
│           ├── test_security_workflow.rs
│           └── common/
└── docs/
```

## Unit Tests

Unit tests stay next to the code they exercise. Use them for fast validation of pure logic and
crate-local behavior.

```bash
# All library/unit tests
cargo test --workspace --lib

# A single crate
cargo test -p openrustclaw-core --lib
```

## Integration Tests

The integration crate is where OpenRustClaw validates stitched-together behavior across crates and
operator-facing command/data APIs.

Notable coverage already in the crate:

- agent runtime and gateway integration
- MCP and provider-chain behavior
- memory and scheduler workflows
- security flows
- fixture-driven shipped-surface checks

The `fixture_suite_test` module is the fast local fixture lane for:

- media inspect and text extraction
- mobile pairing and unpair flows
- mobile command approval and rejection flows
- durable session persistence and status transitions

These tests should avoid real external services and use tempdirs, in-memory state, or local SQLite
fixtures whenever possible.

```bash
# Full integration crate
cargo test -p openrustclaw-integration-tests --quiet

# Just the fixture suite
cargo test -p openrustclaw-integration-tests fixture_suite_test --quiet
```

## E2E Tests

The E2E crate holds the slower workflow-oriented tests and the standalone `e2e` runner summary.

```bash
# Full E2E crate
cargo test -p openrustclaw-e2e-tests --quiet

# Specific workflow file
cargo test -p openrustclaw-e2e-tests test_scheduler_workflow --quiet

# Show the standalone runner summary
cargo run -p openrustclaw-e2e-tests --bin e2e
```

`E2E_LIVE=1` is only for the standalone runner or tests that are explicitly written to opt into
live provider validation.

## Expectations

- Prefer deterministic fixtures over live network calls.
- Add or update docs when the test layout changes.
- Do not mark roadmap parity items complete unless the relevant tests exist and pass.
- Keep operator-surface tests close to the real command/data APIs so docs, CLI help, and runtime
  behavior stay aligned.
