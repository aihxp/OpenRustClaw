# OpenRustClaw

OpenRustClaw is a self-hosted open-source assistant platform built around a Rust-first runtime, durable operator surfaces, and a trust-first control plane. It is meant to be one product that can scale from a solo deployment to a team, company, or enterprise-managed installation without changing the core story.

## Start Here

If you are new to the project:

1. Read [Installation](./getting-started/installation.md)
2. Follow [Quickstart](./getting-started/quickstart.md)
3. Use [First Agent](./getting-started/first-agent.md) once the base runtime is working

If you are evaluating the shipped product surface:

- [Roadmap](./planning/roadmap.md)
- [Feature Matrix](./planning/feature-matrix.md)
- [Surface Matrix](./planning/surface-matrix.md)
- [Product Positioning](./planning/product-positioning.md)
- [Documentation Contract](./planning/documentation-contract.md)

## What OpenRustClaw Already Ships

- Guided onboarding with resumable setup state, repair paths, and setup handoff
- Persisted assistant continuity with memory-write policy and inspectable session state
- Shared CLI, HTTP, MCP, and Control UI operator surfaces
- Tools, coding, browser, channel, voice, mobile, and runtime inspection workflows
- Enterprise access, policy, governance, audit export, and operator-gated autonomy controls

## How to Read the Docs

- `Getting Started` is for new operators bringing up a workspace
- `Deployment`, `Operations`, and `Guides` are for running and extending the system
- `Planning` exposes the canonical shipped-surface references
- `API Reference` documents core runtime contracts

The root docs under `docs/` remain canonical for the planning-facing matrices and positioning pages. The mdBook pages under `docs/src/planning/` are entry points into that canonical set.
