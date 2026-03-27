# Phase 27: Self-Hosted Product Surface Alignment - Context

**Gathered:** 2026-03-27
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 24 introduced the self-hosted product-mode contract, Phase 25 made onboarding choose it explicitly, and Phase 26 added upgrade or downgrade transitions. The remaining gap is surface alignment: the shipped behavior is clearer than the top-level product docs.

This phase should:

- present OpenRustClaw consistently as a self-hosted open-source product
- explain the supported deployment paths: solo, multi-user team, company, enterprise
- show that onboarding chooses a deployment path up front
- show that operators can later upgrade or downgrade through shipped control surfaces

</domain>

<decisions>
## Implementation Decisions

### Keep this phase narrow and operator-facing
Do not invent new lifecycle features here. Align the existing README, install, and getting-started guides with the product-mode and transition behavior that already ships.

### Reuse the shipped Self-Hosted Product Mode surface
The runtime and Control UI already expose the current mode and recent transitions. The docs should point to that surface instead of describing speculative future tooling.

### Prefer truthful copy over marketing expansion
This phase should make the product easier to understand without broadening the milestone scope beyond self-hosted packaging and lifecycle clarity.

</decisions>

<code_context>
## Existing Code Insights

- `crates/cli/src/commands/self_hosted.rs` already owns the product-mode and transition contract.
- `crates/cli/src/commands/inspect.rs` and `crates/cli/src/commands/start.rs` already expose typed runtime summaries for the current mode.
- `crates/cli/src/commands/control_ui.html` already renders the `Self-Hosted Product Mode` panel with current mode, warnings, and recent transitions.
- `README.md`, `docs/src/getting-started/installation.md`, `docs/src/getting-started/quickstart.md`, and `docs/src/getting-started/first-agent.md` are the highest-signal user-facing guides still needing alignment.

</code_context>

<specifics>
## Specific Ideas

- update README quickstart copy to frame OpenRustClaw as self-hosted and mode-aware
- add a short deployment-path chooser section to installation and quickstart
- document post-install upgrade or downgrade through `/control/ui` and `/control/self-hosted/product-mode`
- tighten the shipped dashboard copy so the self-hosted panel reads like the same product story as the docs

</specifics>

<deferred>
## Deferred Ideas

- pricing, commercial packaging, or hosted-cloud messaging
- deep onboarding screenshots or separate role-specific manuals
- approval-chain or policy changes for product-mode transitions

</deferred>
