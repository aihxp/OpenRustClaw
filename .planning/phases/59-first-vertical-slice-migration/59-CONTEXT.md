# Phase 59: First Vertical Slice Migration - Context

**Gathered:** 2026-03-28
**Status:** Ready for execution

## Goal

Migrate one high-value shipped vertical slice into the new architecture lane to prove the transition with production code.

## What We Know

- Phase 57 selected setup handoff reporting as the first proving slice because it already spans setup-state persistence, report composition, one runtime route, and one Control UI panel.
- Phase 58 introduced `openrustclaw-app` with a stable `setup_handoff` service contract, but the shipped CLI still builds the report directly in `crates/cli/src/commands/inspect.rs`.
- `start.rs` and the Control UI already consume the report through `inspect::setup_handoff_summary`, so the migration can stay bounded if the JSON shape is preserved.
- Existing tests already cover the default no-state case and a degraded saved-state case for the report path.

## Constraints

- This phase must preserve the outward setup handoff contract for the runtime route and Control UI.
- The migrated path should reduce CLI-owned business logic, not just wrap the same logic in another helper.
- The phase should keep the source-of-truth setup manifest in the CLI onboarding module for now, because broader persistence movement is outside this proving slice.

## Implementation Direction

- make `openrustclaw-app` the report-building owner for setup handoff
- add a narrow CLI adapter that loads onboarding state and maps it into the new application-layer service
- keep the route and UI surfaces unchanged while verification proves the migrated slice still behaves the same
