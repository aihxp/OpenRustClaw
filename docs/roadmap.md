# OpenRustClaw Roadmap

This file is the canonical planning-facing record for the shipped OpenRustClaw product surface. It is not a feature wishlist. It is the statement of what the current self-hosted product already treats as part of its supported runtime and operator contract.

## Current Product Baseline

OpenRustClaw now ships:

- a self-hosted product model across `solo`, `team`, `company`, and `enterprise`
- a Rust-first production runtime with the sidecar kept as an optional compatibility lane
- guided onboarding, resumable setup, repair, and setup handoff
- persisted assistant continuity, memory policy, and operator-visible evidence
- shared CLI, HTTP, MCP, and Control UI surfaces
- enterprise access, policy, governance, audit, and operator-gated autonomy controls

## Historical Foundation

The shipped baseline was built through these program slices:

1. MVP trust baseline
2. lifecycle integrity and verification archive
3. deeper operator parity across browser, supervision, mobile, Control UI, and voice
4. enterprise identity, policy, governance, and autonomy foundations
5. self-hosted product modes and upgrade or downgrade lifecycle
6. truthful onboarding, repair, and setup handoff

Those slices are now part of the declared product baseline, not pending roadmap items.

## How to Use This File

Use this page with:

- [feature-matrix.md](feature-matrix.md)
- [surface-matrix.md](surface-matrix.md)
- [product-positioning.md](product-positioning.md)
- [docs-audit.md](docs-audit.md)
- [documentation-contract.md](documentation-contract.md)

Together they define:

- what OpenRustClaw can fairly claim today
- which surfaces are canonical planning references
- which docs and tests back the shipped contract
- where the product intentionally stays bounded

## Roadmap Rules

- a feature is only part of the shipped surface when runtime behavior, operator visibility, and docs all agree
- a bounded or gated lane should be described as bounded or gated, not promoted into default behavior
- new milestones should expand the product from the current truthful baseline instead of reopening retired internal programs

## Forward Direction

Future work should be framed as incremental expansion or hardening on top of the current baseline, for example:

- broader enterprise packaging
- deeper autonomy in bounded domains
- additional parity or operator polish
- stronger release automation and operational hardening

The baseline itself already exists. The roadmap now protects that truth instead of speculating past it.
