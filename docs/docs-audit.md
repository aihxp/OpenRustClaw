# OpenRustClaw Docs Audit

Reviewed: 2026-03-25

This is the repo-root source-of-truth audit for the final Phase 8 docs/test gate.

It maps the shipped feature families to:

- user-facing docs,
- operator-facing docs,
- troubleshooting coverage,
- representative automated tests.

Unsupported or intentionally excluded surfaces are not counted here. Gaps that stay
out of the shipped surface remain tracked in the surface matrix instead of this audit.

| Feature family | User docs | Operator docs | Troubleshooting / operational guidance | Representative tests |
| --- | --- | --- | --- | --- |
| Gateway runtime and deployment | `README.md`, `docs/src/getting-started/quickstart.md` | `docs/src/deployment/production.md`, `docs/src/deployment/docker.md` | `docs/src/operations/observability.md`, `docs/src/deployment/production.md` | `tests/integration/src/gateway_test.rs`, `tests/e2e/tests/vertical/test_gateway_layer.rs`, `scripts/check-runtime-budgets.sh` |
| Channels and routing | `README.md`, `docs/feature-matrix.md` | `docs/src/guides/security.md`, `docs/src/deployment/production.md` | `docs/src/guides/security.md`, `docs/src/operations/observability.md` | `tests/integration/src/channel_fixture_test.rs`, `tests/integration/src/documented_scenario_test.rs`, `crates/cli/src/commands/channels.rs` unit tests |
| Sessions and routing policy | `README.md`, `docs/feature-matrix.md` | `docs/roadmap.md`, `docs/surface-matrix.md` | `docs/src/operations/observability.md`, `docs/src/contributing/testing.md` | `tests/integration/src/fixture_suite_test.rs`, `tests/integration/src/documented_scenario_test.rs`, `crates/cli/src/commands/start.rs` unit tests |
| Media workflows and extraction | `README.md`, `docs/feature-matrix.md` | `docs/src/contributing/testing.md`, `docs/surface-matrix.md` | `docs/src/operations/observability.md`, `docs/src/guides/providers.md` | `tests/integration/src/fixture_suite_test.rs`, `tests/integration/src/documented_scenario_test.rs`, `cargo test -p openrustclaw-cli media --quiet` |
| Mobile pairing and node commands | `README.md`, `docs/feature-matrix.md` | `docs/src/deployment/production.md`, `docs/surface-matrix.md` | `docs/src/operations/observability.md`, `docs/src/guides/security.md` | `tests/integration/src/fixture_suite_test.rs`, `tests/integration/src/documented_scenario_test.rs`, `cargo test -p openrustclaw-cli mobile --quiet` |
| Skills, extensions, and MCP tools | `README.md`, `docs/src/guides/skills.md`, `docs/src/guides/mcp-servers.md` | `docs/product-positioning.md`, `docs/surface-matrix.md` | `docs/src/guides/security.md`, `docs/src/operations/observability.md` | `tests/integration/src/mcp_test.rs`, `cargo test -p openrustclaw-cli skills --quiet`, `cargo test -p openrustclaw-cli mcp_server --quiet` |
| Providers and fallback health | `README.md`, `docs/src/guides/providers.md` | `docs/src/operations/observability.md`, `docs/roadmap.md` | `docs/src/guides/providers.md`, `docs/src/operations/observability.md` | `tests/integration/src/provider_chain_test.rs`, `cargo test -p openrustclaw-cli runtime --quiet`, `cargo test -p openrustclaw-providers --quiet` |
| Control plane and operator security | `README.md`, `docs/src/guides/security.md` | `docs/src/deployment/production.md`, `docs/surface-matrix.md` | `docs/src/guides/security.md`, `docs/src/operations/observability.md` | `tests/integration/src/security_test.rs`, `tests/integration/src/documented_scenario_test.rs`, `crates/cli/src/commands/start.rs` unit tests |
| Runtime operations and releases | `README.md`, `docs/src/getting-started/installation.md` | `docs/src/deployment/production.md` | `docs/src/deployment/production.md`, `docs/src/operations/observability.md` | `cargo test -p openrustclaw-cli runtime --quiet`, `scripts/build-release-artifacts.sh`, `scripts/check-runtime-budgets.sh` |
| Observability and diagnostics | `docs/src/operations/observability.md` | `docs/src/deployment/production.md`, `docs/feature-matrix.md` | `docs/src/operations/observability.md` | `cargo test -p openrustclaw-observability --quiet`, `cargo test -p openrustclaw-gateway --quiet`, `cargo test -p openrustclaw-cli start --quiet` |

## Audit Result

Supported feature families now have:

- at least one user-facing doc entry,
- at least one operator-facing or deployment-facing doc entry,
- at least one troubleshooting or operational reference,
- at least one representative automated test lane.

This audit is the artifact backing the Phase 8 docs/test exit check. Future shipped feature
families should be added here in the same commit that adds the feature surface.
