# Requirements: OpenRustClaw

## Active Milestone: v1.15 Deeper Greenfield Conversion

**Goal:** Continue the greenfield transition by moving the next meaningful inspection, route, runtime, and mutation-heavy skills seams into cleaner application or adapter boundaries without weakening the shipped operator surface.

## Requirements

### GFC-05 Inspection aggregation expansion

OpenRustClaw must move another real inspection or aggregation family out of `crates/cli/src/commands/inspect.rs` so the greenfield lane keeps absorbing operator-facing report composition instead of stalling after the first few migrated summaries.

**Status:** Completed in v1.15 Phase 65 via the enterprise admin aggregation service.

**Acceptance signals:**
- at least one additional inspection or aggregation family is composed in `openrustclaw-app`
- `inspect.rs` becomes an adapter for that migrated surface instead of owning the business rules directly
- existing report-facing verification remains truthful

### GFC-06 Additional route-family extraction

Another bounded runtime or `/control/...` route family must stop coupling directly to mixed command-local logic and instead call a stable application-facing service or adapter boundary.

**Status:** Completed in v1.15 Phase 66 via the enterprise access control route family.

**Acceptance signals:**
- one real route family moves behind a cleaner service seam
- route behavior remains stable from the runtime API perspective
- the migration reduces direct cross-calls from `start.rs` or another route-heavy legacy module into unrelated command helpers

### GFC-07 Skills mutation and registry boundary

The mutation-heavy `skills.rs` lanes must gain a real service boundary so future skills work stops treating the compiled overview seam as the only extraction while install, mutation, and registry behavior keep growing inside the hotspot.

**Acceptance signals:**
- one real mutation-heavy slice of `skills.rs` moves behind a stable service or adapter boundary
- the milestone starts from compiled-skill mutation, removal, install, or registry flows instead of reopening the already-migrated overview seam
- contributor guidance can point future skills work at the new seam

### GFC-08 Runtime command boundary cleanup

At least one remaining runtime command seam must move behind a cleaner application or adapter boundary so the greenfield transition does not stay concentrated only in report composition and route wrappers.

**Acceptance signals:**
- one bounded runtime command or operator-control seam is migrated out of a legacy command hub
- the shipped CLI or runtime contract remains intact
- verification proves the migrated runtime path still behaves truthfully

## Most Recent Archive

- Last shipped milestone: `v1.14 Continued Greenfield Conversion`
- Archived requirements: `.planning/milestones/v1.14-REQUIREMENTS.md`
- Archived verification bundle: `.planning/milestones/v1.14-VERIFICATIONS.md`

## Next Step

Continue with `$gsd-discuss-phase 67`, `$gsd-plan-phase 67`, or `$gsd-autonomous`.
