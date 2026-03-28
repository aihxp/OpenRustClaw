# Requirements: OpenRustClaw

## Active Milestone: v1.14 Continued Greenfield Conversion

**Goal:** Continue the greenfield transition by moving additional shipped behavior out of legacy CLI command hubs and into `openrustclaw-app`, while preserving truthful operator behavior and narrowing the next highest-risk hotspots.

## Requirements

### GFC-01 Inspection summary service extraction

OpenRustClaw must move broader inspection-summary composition out of `crates/cli/src/commands/inspect.rs` and into the greenfield application lane so typed operator summaries stop being owned primarily by a legacy CLI command hub.

**Acceptance signals:**
- at least one additional typed inspection family is composed in `openrustclaw-app`
- `inspect.rs` becomes an adapter for that migrated summary instead of owning the business rules directly
- existing report-facing verification remains truthful

### GFC-02 Control route service seam

Selected `/control/...` route families in `crates/cli/src/commands/start.rs` must stop coupling directly to mixed CLI command logic and instead call stable application-facing services or adapters around them.

**Acceptance signals:**
- one bounded route family moves behind a cleaner service boundary
- route behavior remains stable from the runtime API perspective
- the migration reduces direct cross-calls from `start.rs` into mixed legacy command logic

### GFC-03 Mobile report migration

At least one mobile operator-reporting surface must follow the proving-slice pattern and run through the greenfield application lane.

**Acceptance signals:**
- one real mobile operator report is built by `openrustclaw-app`
- the shipped Control UI or runtime route contract remains intact
- verification proves the migrated mobile surface still behaves truthfully

### GFC-04 Skills boundary cleanup

The `skills.rs` hotspot must gain a bounded service seam so future work has a cleaner place to land than the legacy command module itself.

**Acceptance signals:**
- one meaningful slice of `skills.rs` is extracted behind a stable service or adapter boundary
- contributor guidance can point future work at that seam instead of the hotspot
- the milestone preserves the next cleanup queue honestly instead of claiming the file is fully solved

## Most Recent Archive

- Last shipped milestone: `v1.13 Brownfield-to-Greenfield Transition`
- Archived requirements: `.planning/milestones/v1.13-REQUIREMENTS.md`
- Archived verification bundle: `.planning/milestones/v1.13-VERIFICATIONS.md`

## Next Step

Start execution with `$gsd-plan-phase 61` or `$gsd-autonomous`.
