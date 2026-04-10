# OpenRustClaw Roadmap

This file is the canonical planning-facing record for the shipped OpenRustClaw product surface. It is not a feature wishlist. It is the statement of what the current self-hosted product already treats as part of its supported runtime and operator contract.

## Current Product Baseline

OpenRustClaw now ships:

- a self-hosted product model across `solo`, `team`, `company`, and `enterprise`
- a Rust-first production runtime with the sidecar kept as an optional compatibility lane
- guided onboarding, resumable setup, repair, and setup handoff with provider, access-mode, and primary-model continuity
- persisted assistant continuity, memory policy, and operator-visible evidence
- bounded learning, structured memory artifacts, and an operator-gated God Mode lane
- truthful direct-provider, local-runtime, and delegated local-agent selection during onboarding and runtime inspection
- trusted remote delegated backends, durable route receipts, and a dedicated routing console
- guided first-task launch that stays aligned with the selected lane and route policy
- Tailscale-first private gateway guidance with truthful bound-versus-advertised runtime addresses
- runtime lifecycle control through `openrustclaw start`, `openrustclaw stop`, and `openrustclaw restart`, plus startup preflight checks for release updates and safe OpenClaw migration
- a public semver release lane for `openrustclaw-core` through `1.4.10`, kept separate from milestone archive tags
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
- how milestone archive tags differ from the public semver release line

## Roadmap Rules

- a feature is only part of the shipped surface when runtime behavior, operator visibility, and docs all agree
- a bounded or gated lane should be described as bounded or gated, not promoted into default behavior
- new milestones should expand the product from the current truthful baseline instead of reopening retired internal programs
- public semver releases and planning milestone tags should be treated as separate contracts with explicit docs and metadata for each

## Forward Direction

Future work should be framed as incremental expansion or hardening on top of the current baseline, for example:

- broader enterprise packaging
- deeper autonomy in bounded domains
- additional parity or operator polish
- stronger release automation and operational hardening
- richer delegated-fabric economics, saturation, and route simulation

The baseline itself already exists. The roadmap now protects that truth instead of speculating past it.
