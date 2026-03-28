# Phase 75: Runtime Upgrade and Rollback Planning Boundary - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning

## Phase Boundary

Move one larger runtime upgrade, self-update, or rollback planning seam behind a cleaner application service so `runtime.rs` keeps shrinking beyond provider switching, vault mutation, and reload planning.

## Decisions

- Upgrade, self-update, and rollback planning should move together because they share blocker detection, restart guidance, and maintenance-step generation.
- `runtime.rs` should remain the adapter that loads runtime status, health, reload-state, service-manager state, lock state, and artifact metadata.
- The planning service should own only the decision logic and generated operator steps, leaving file I/O and process inspection in the CLI adapter.

## Existing Code Insights

- `runtime_upgrade_plan`, `runtime_self_update_plan`, and `runtime_rollback_plan` currently build blocker lists and operator steps inline.
- `start.rs` already exposes `/control/runtime/upgrade-plan`, `/control/runtime/self-update-plan`, and `/control/runtime/rollback-plan`, so a shared planning service creates the right follow-on route-family seam for Phase 76.
