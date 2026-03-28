# Phase 62: Control Route Family Service Extraction - Context

**Gathered:** 2026-03-28
**Status:** Ready for execution

## Goal

Reduce direct business-logic ownership inside selected `start.rs` control route families by moving them behind cleaner application-facing services.

## What We Know

- `v1.14` Phase 61 already moved self-hosted product-mode summary composition into `openrustclaw-app`, so the next truthful route-family extraction can build on that same product-mode surface.
- The `/control/self-hosted/product-mode` family is already bounded to one GET summary path and one POST transition path, which makes it a cleaner proving target than a broader runtime or enterprise route cluster.
- The current POST handler in `start.rs` still owns the transition orchestration directly by calling `self_hosted::transition_mode(...)`, recording the tool result, and then re-fetching the summary.
- Persisted product-mode storage and transition receipts can stay in the CLI adapter module for now; this phase is about moving route-facing orchestration behind a cleaner service seam.

## Constraints

- The migrated route family must preserve the existing runtime API response shape and the shipped Control UI behavior.
- This phase should reduce direct cross-calls from `start.rs` into `self_hosted.rs` without broadening scope into storage migration or route reorganization.
- Verification needs one focused regression signal for the migrated transition path, not just the existing summary-only tests.

## Implementation Direction

- extend the self-hosted product-mode service boundary in `openrustclaw-app` so it can own the transition-and-report use case
- keep `inspect.rs` as the workspace adapter that bridges persisted CLI storage into the new service contract
- reduce `start.rs` to a thin HTTP adapter that delegates the self-hosted product-mode transition route to the migrated seam
