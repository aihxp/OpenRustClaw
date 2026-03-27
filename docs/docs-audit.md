# OpenRustClaw Docs Audit

Reviewed: 2026-03-27

This is the canonical documentation and test-coverage audit for the shipped OpenRustClaw surface. It answers two questions:

1. does each major shipped feature family have real user-facing and operator-facing documentation
2. are future milestones updating the canonical docs set together instead of creating drift

Use this audit with:

- [documentation-contract.md](documentation-contract.md)
- [roadmap.md](roadmap.md)
- [feature-matrix.md](feature-matrix.md)
- [surface-matrix.md](surface-matrix.md)

## Feature-Family Coverage

| Feature family | User docs | Operator docs | Troubleshooting / operational guidance | Representative tests |
| --- | --- | --- | --- | --- |
| Gateway runtime and deployment | `README.md`, `docs/src/getting-started/quickstart.md` | `docs/src/deployment/production.md` | `docs/src/operations/observability.md`, `docs/src/deployment/production.md` | `tests/integration/src/gateway_test.rs`, `tests/e2e/tests/vertical/test_gateway_layer.rs`, `scripts/check-runtime-budgets.sh` |
| Setup and onboarding | `README.md`, `docs/src/getting-started/installation.md`, `docs/src/getting-started/quickstart.md` | `docs/src/deployment/production.md` | `docs/src/getting-started/installation.md`, `docs/src/guides/security.md` | `cargo test -p openrustclaw-cli onboard -- --nocapture`, `cargo test -p openrustclaw-cli doctor -- --nocapture` |
| Sessions, memory, and continuity | `README.md`, `docs/src/getting-started/quickstart.md`, `docs/src/getting-started/first-agent.md` | `docs/src/operations/observability.md`, `docs/feature-matrix.md` | `docs/src/operations/observability.md`, `docs/src/guides/security.md` | `tests/integration/src/fixture_suite_test.rs`, `tests/integration/src/documented_scenario_test.rs`, `crates/cli/src/commands/start.rs` unit tests |
| Tools, browser, and coding evidence | `README.md`, `docs/src/getting-started/first-agent.md` | `docs/src/operations/observability.md`, `docs/feature-matrix.md` | `docs/src/operations/observability.md` | `tests/integration/src/mcp_test.rs`, `cargo test -p openrustclaw-cli skills --quiet`, `cargo test -p openrustclaw-cli mcp_server --quiet` |
| Channels, communications, voice, and mobile | `README.md`, `docs/feature-matrix.md` | `docs/src/deployment/production.md`, `docs/surface-matrix.md` | `docs/src/operations/observability.md`, `docs/src/guides/security.md` | `tests/integration/src/channel_fixture_test.rs`, `tests/integration/src/documented_scenario_test.rs`, `cargo test -p openrustclaw-cli mobile --quiet` |
| Enterprise and autonomy controls | `README.md`, `docs/product-positioning.md` | `docs/src/deployment/production.md`, `docs/src/guides/security.md`, `docs/surface-matrix.md` | `docs/src/operations/observability.md`, `docs/src/deployment/production.md` | `cargo test -p openrustclaw-cli enterprise_access -- --nocapture`, `cargo test -p openrustclaw-cli enterprise_autonomy -- --nocapture` |
| Runtime operations, release, and security posture | `README.md`, `docs/src/getting-started/installation.md` | `docs/src/deployment/production.md`, `docs/src/guides/security.md` | `docs/src/operations/observability.md`, `docs/src/deployment/release-checklist.md` | `cargo test -p openrustclaw-cli runtime --quiet`, `scripts/build-release-artifacts.sh`, `scripts/check-runtime-budgets.sh` |

## Drift-Prevention Checklist

When a future milestone changes shipped behavior, verify that it updates:

- `README.md` if the product story, first-run path, or primary operator loop changed
- the relevant mdBook guide under `docs/src/`
- any affected canonical planning doc under `docs/`
- this audit if a shipped feature family gained, lost, or materially changed documentation coverage

If a milestone changes behavior but cannot name the canonical docs it updated, the docs work is incomplete.

## Audit Result

The rewritten docs set now has:

- one explicit documentation ownership contract
- one clearer repo and mdBook entry story
- one aligned getting-started path
- one compact operator-facing runbook set
- one standing audit artifact for future drift checks

