# Phase 57: Greenfield Boundary Contract and Migration Inventory - Context

**Gathered:** 2026-03-28
**Status:** Ready for execution

## Goal

Define the clean architecture contract, ownership rules, no-touch seams, and first migration targets that turn the repo from opportunistic brownfield change into deliberate greenfield transition work.

## What We Know

- The workspace already has a strong crate foundation, but the largest coupling pressure still lives in the CLI and control-plane command layer.
- `crates/cli/src/commands/start.rs` remains the largest hotspot at roughly 13.6k lines, with `mobile.rs`, `skills.rs`, `orchestrate.rs`, `runtime.rs`, and `inspect.rs` also acting as mixed command and application logic hubs.
- `openrustclaw-core` already provides the lowest-level shared types and traits, which makes it the right foundation for a greenfield application lane rather than inventing a second domain base.
- The setup handoff flow is a realistic first proving slice because it already spans onboarding state, inspection, a control route, and Control UI rendering, while still having focused regression tests.

## Constraints

- This phase must not pretend that the brownfield system disappears; it needs to define containment and migration rules around the existing shipped product.
- The first migration target should be small enough to move without destabilizing the control plane, but broad enough to prove a real architectural seam.
- The canonical contract should land in both planning-facing and contributor-facing documentation so future work can actually follow it.

## Implementation Direction

- add one canonical planning contract that defines the target layer model, legacy containment surfaces, and first migration inventory
- add one contributor-facing architecture page that explains how the greenfield lane is supposed to work inside the current repo
- update architecture and development docs so new work defaults to the new lane instead of deepening legacy command modules
- record the first proving slice explicitly around setup handoff reporting and remote-connectivity setup state
