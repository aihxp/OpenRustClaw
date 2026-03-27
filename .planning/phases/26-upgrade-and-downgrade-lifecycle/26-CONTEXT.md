# Phase 26: Upgrade and Downgrade Lifecycle - Context

**Gathered:** 2026-03-27
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 24 introduced the product-mode contract and Phase 25 made onboarding write it. Phase 26 should make changing that contract explicit and reviewable instead of letting operators overwrite product-mode state silently.

This phase should:

- support explicit upgrade or downgrade transitions between the supported self-hosted modes
- record transition history durably
- surface retained-data or governance warnings when moving down from stronger modes
- expose the transition path through shipped runtime and Control UI surfaces

</domain>

<decisions>
## Implementation Decisions

### Keep transitions additive and auditable
Do not delete enterprise data on downgrade. Record warnings and keep retained state explicit so the operator can clean it up deliberately.

### Reuse the existing product-mode manifest
The lifecycle path should extend the same manifest and summary contract rather than inventing a second mode-transition system.

### Use Control UI as the first shipped operator loop
The request explicitly included upgrade or downgrade ability, so the runtime API and shipped dashboard should expose it directly instead of requiring file edits.

</decisions>

<code_context>
## Existing Code Insights

- `crates/cli/src/commands/self_hosted.rs` already owns the product-mode manifest and can absorb event history plus transition logic.
- `crates/cli/src/commands/inspect.rs` already summarizes the product-mode contract.
- `crates/cli/src/commands/start.rs` and `crates/cli/src/commands/control_ui.html` already form the shipped operator loop for similar enterprise and autonomy surfaces.

</code_context>

<specifics>
## Specific Ideas

- add a product-mode transition request and JSONL event ledger
- classify changes as upgrades vs downgrades
- warn when enterprise access or full-autonomy data remains on disk after a downgrade
- expose `POST /control/self-hosted/product-mode` as the transition action
- add a transition form and recent-transition table to the Control UI panel

</specifics>

<deferred>
## Deferred Ideas

- automatic cleanup of enterprise manifests during downgrade
- approval chains for product-mode transitions
- release-note or docs alignment beyond the shipped UI and API surface

</deferred>
