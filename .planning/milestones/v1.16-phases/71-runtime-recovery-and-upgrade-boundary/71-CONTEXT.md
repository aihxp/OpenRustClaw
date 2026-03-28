# Phase 71: Runtime Recovery and Upgrade Boundary - Context

**Gathered:** 2026-03-28
**Status:** Ready for execution

## Goal

Move one larger runtime recovery-oriented seam behind a cleaner application service so the greenfield transition expands beyond the first bounded provider-switch lane.

## What We Know

- `crates/cli/src/commands/runtime.rs` still owns the reload-planning orchestration inline through `runtime_reload_plan(...)`.
- That path compares the current runtime snapshot against the applied snapshot, classifies live-reload-safe deltas versus restart-required changes, and feeds both operator-facing runtime APIs and higher-level recovery summaries.
- The greenfield win here is to move reload-plan computation into `openrustclaw-app` while keeping `runtime.rs` as the adapter around snapshot capture and persisted reload-state I/O.

## Constraints

- This phase must extract one larger runtime recovery or planning seam, not just wrap a tiny helper.
- The shipped runtime reload-plan contract must stay stable for both CLI and control API consumers.
- The migration should preserve a clear follow-on path for the remaining upgrade-plan and route-family work still coupled to `runtime.rs`.

## Implementation Direction

- add a runtime reload-planning service to `openrustclaw-app`
- move snapshot comparison, restart classification, and reload-plan result shaping behind that service
- keep `runtime.rs` as the adapter that captures snapshots and loads or saves reload-state artifacts
