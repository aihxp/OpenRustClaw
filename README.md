# OpenRustClaw

[![CI](https://github.com/aihxp/OpenRustClaw/actions/workflows/ci.yml/badge.svg)](https://github.com/aihxp/OpenRustClaw/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/rust-2024_edition-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

OpenRustClaw is a self-hosted open-source assistant platform built around a Rust-first runtime, durable operator surfaces, and a trust-first control plane. It is designed for people who want one deployable assistant product they can run for themselves, a small team, a company, or an enterprise environment without depending on a hosted SaaS control layer.

The product already ships a coherent baseline for onboarding, persisted assistant continuity, memory policy, tools and coding audit trails, browser and channel workflows, voice and mobile operator surfaces, enterprise governance, and an operator-gated full-autonomy lane. The current documentation milestone is about making that shipped surface legible.

## Choose Your Path

OpenRustClaw is packaged as one product with four deployment modes:

| Mode | Best fit | Typical first step |
| --- | --- | --- |
| `solo` | one operator, one workspace, local or private host | run `openrustclaw onboard` and choose `Standard` |
| `team` | a shared assistant for a small internal group | start with guided onboarding, then review `/control/ui` |
| `company` | shared runtime with stronger operational controls | use guided onboarding, then follow the production docs |
| `enterprise` | operator-managed deployment with access, policy, audit, and autonomy controls | bootstrap through onboarding, then review enterprise and production surfaces |

The setup flow supports `Standard`, `Advanced`, and `Custom` depth, can resume partially completed work, and supports explicit upgrade or downgrade transitions between deployment modes.

## What Ships Today

- Guided self-hosted onboarding with durable setup state, repair, and setup handoff
- Persisted assistant chat with session continuity and explicit memory-write policy
- Shared CLI, HTTP, MCP, and Control UI operator surfaces
- Bounded tools, coding audit trails, browser workflows, and runtime inspection
- Multi-channel and communications coverage with operator-visible evidence
- Voice, mobile, and enterprise operator summaries and controls
- Enterprise access, policy, governance, audit export, and operator-gated full autonomy

For the detailed shipped-surface record, use the canonical planning docs under [`docs/`](docs/).

## Quick Start

### 1. Build the workspace

```bash
cargo build --workspace
cp .env.example .env
```

Add at least one provider API key to `.env`.

### 2. Run guided onboarding

```bash
cargo run --bin openrustclaw -- onboard
```

The onboarding flow asks which deployment mode you want, offers `Standard`, `Advanced`, or `Custom` setup depth, validates provider and runtime readiness, and only offers to launch the assistant after the first-start health gate passes.

### 3. Start the runtime and assistant

```bash
cargo run --bin openrustclaw -- start
cargo run --bin openrustclaw -- assistant
```

Once the runtime is up, open `/control/ui` to inspect the same workspace through the shipped operator surface.

## Operator Loop

The normal operator loop is:

1. `openrustclaw onboard` to choose a deployment path and configure the workspace
2. `openrustclaw doctor` to verify first-start or repair blockers
3. `openrustclaw start` to run the Rust-owned runtime
4. `openrustclaw assistant` for the persisted assistant session
5. `/control/ui` for sessions, setup handoff, product mode, tools, browser, enterprise, autonomy, and runtime inspection

Useful commands:

```bash
openrustclaw doctor
openrustclaw session list
openrustclaw session show <session-id>
openrustclaw runtime status
openrustclaw memory timeline --limit 10
```

## Documentation

Use these as the main entry points:

| Need | Start here |
| --- | --- |
| Install and configure a workspace | [docs/src/getting-started/installation.md](docs/src/getting-started/installation.md) |
| Get running quickly | [docs/src/getting-started/quickstart.md](docs/src/getting-started/quickstart.md) |
| Production deployment and operations | [docs/src/deployment/production.md](docs/src/deployment/production.md) |
| Security model and operator guidance | [docs/src/guides/security.md](docs/src/guides/security.md) |
| Observability and runtime signals | [docs/src/operations/observability.md](docs/src/operations/observability.md) |
| Canonical docs ownership rules | [docs/documentation-contract.md](docs/documentation-contract.md) |
| Shipped-surface planning references | [docs/roadmap.md](docs/roadmap.md), [docs/feature-matrix.md](docs/feature-matrix.md), [docs/surface-matrix.md](docs/surface-matrix.md), [docs/product-positioning.md](docs/product-positioning.md) |

The mdBook navigation under [`docs/src/`](docs/src/) is for guided reading. The root planning docs under [`docs/`](docs/) remain the canonical source for the shipped-surface matrices and positioning pages.

## Product Boundaries

OpenRustClaw is intentionally opinionated:

- it is self-hosted and open source, not a hosted SaaS assistant
- the Rust runtime is the default production path
- the Python sidecar is an optional compatibility lane, not the required runtime core
- full autonomy is a separate operator-gated lane, not the default behavior
- documentation should describe shipped behavior, not aspirational scope

## Development

```bash
cargo build --workspace
cargo test --workspace
cargo fmt --all
cargo clippy --workspace --all-targets
mdbook build docs
```

## License

OpenRustClaw is released under the [MIT License](LICENSE).
