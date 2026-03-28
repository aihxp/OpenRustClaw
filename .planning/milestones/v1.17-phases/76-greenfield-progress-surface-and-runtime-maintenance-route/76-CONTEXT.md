# Phase 76: Greenfield Progress Surface and Runtime Maintenance Route - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning

## Phase Boundary

Expose the greenfield completion percentage and remaining ranked queue through one shipped inspect or `/control/...` surface while moving one more bounded runtime-maintenance family behind the application services introduced by this milestone.

## Decisions

- The shipped surface should use the canonical ranked seam inventory instead of milestone-count progress so operator-visible progress remains truthful.
- A bounded runtime-maintenance summary route is a safer follow-on seam than rewriting all maintenance handlers at once because Phase 75 already extracted the shared planning logic they depend on.
- The progress inventory must be updated as later phases land; otherwise the new surface would ship stale completion numbers immediately.

## Existing Code Insights

- `openrustclaw-app` already owns the greenfield inventory and the maintenance-planning lane, so a small runtime-maintenance control service can compose the new surface without reintroducing CLI coupling.
- `start.rs` already exposes `/control/runtime/upgrade-plan`, `/control/runtime/self-update-plan`, and `/control/runtime/rollback-plan`, making `/control/runtime/maintenance` a natural bounded summary surface.
- `inspect.rs` already hosts shipped summary helpers, so it can expose the canonical progress report for any future inspect or UI follow-on without duplicating route-local logic.
