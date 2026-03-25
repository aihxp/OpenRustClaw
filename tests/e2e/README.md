# OpenRustClaw E2E Tests

This crate holds the longer-running end-to-end and workflow-oriented suites.

## Current Layout

```text
tests/e2e/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── main.rs
    ├── test_chat_workflow.rs
    ├── test_memory_workflow.rs
    ├── test_provider_fallback.rs
    ├── test_scheduler_workflow.rs
    ├── test_security_workflow.rs
    └── common/
        ├── assertions.rs
        ├── fixtures.rs
        ├── http_client.rs
        └── mod.rs
```

## Running The Crate

```bash
# Run all E2E tests
cargo test -p openrustclaw-e2e-tests --quiet

# Run a specific workflow file
cargo test -p openrustclaw-e2e-tests test_chat_workflow --quiet

# Show the standalone runner summary
cargo run -p openrustclaw-e2e-tests --bin e2e

# Opt into live-provider checks for the standalone runner
E2E_LIVE=1 cargo run -p openrustclaw-e2e-tests --bin e2e
```

## What The E2E Crate Covers

- chat and conversation workflows
- memory storage and recall flows
- scheduler execution behavior
- security boundary checks
- provider fallback behavior

Fast local fixture coverage for media, mobile, and session persistence lives in the separate
`tests/integration/` crate so those paths can run without external services.
