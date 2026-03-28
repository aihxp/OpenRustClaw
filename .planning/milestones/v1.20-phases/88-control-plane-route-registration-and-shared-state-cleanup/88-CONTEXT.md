# Phase 88: Control-Plane Route Registration and Shared State Cleanup - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning

## Phase Boundary

Reduce route-registration sprawl after the v1.19 and v1.20 control-plane extractions so future work stops depending on one oversized `start.rs` route chain to wire migrated control surfaces.

## Decisions

- Keep the cleanup compatibility-preserving: reorganize route registration and shared wiring only, without changing handler contracts.
- Focus the extraction on the migrated control-plane families instead of broad unrelated router churn.
- Prove the cleanup by hitting representative migrated routes after the refactor, rather than relying only on structural inspection.

## Existing Code Insights

- `runtime_control_router` still carried a very large inline route chain even after the moved control-plane families no longer owned inline business logic.
- The migrated families naturally grouped into control-plane routes, non-voice skill-control routes, and voice/channel runtime skill routes.
- Pulling those families behind route-builder helpers materially shrinks the shared setup surface while preserving the existing state wiring model.
