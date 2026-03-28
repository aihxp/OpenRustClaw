# Requirements: OpenRustClaw

## Active Milestone: v1.16 Greenfield Conversion: Skills and Runtime Hotspots

**Goal:** Continue the greenfield transition by migrating the next remaining mutation-heavy `skills.rs` lanes, the next larger runtime command seams, and one more high-value control route family into cleaner application or adapter boundaries without weakening the shipped operator surface.

## Requirements

### GFC-09 Remaining skills plugin lifecycle boundary

The remaining plugin-binding and voice-plugin mutation lanes in `crates/cli/src/commands/skills.rs` must move behind a real service boundary so future plugin-lifecycle work stops accumulating directly in the hotspot.

**Status:** Planned for v1.16 Phase 69.

**Acceptance signals:**
- one real remaining plugin-binding, auth-plugin, or voice-plugin mutation lane moves behind a stable service or adapter boundary
- the migration preserves the shipped CLI and control/runtime mutation contract
- contributor guidance can point future plugin-lifecycle work at the new seam instead of `skills.rs`

### GFC-10 Runtime vault or secret mutation boundary

At least one runtime vault, secret, or equivalent configuration-mutation lane must move out of `crates/cli/src/commands/runtime.rs` and into the greenfield application lane.

**Status:** Planned for v1.16 Phase 70.

**Acceptance signals:**
- one real runtime vault or secret mutation seam is built by `openrustclaw-app`
- the shipped CLI or control API contract remains intact
- verification proves the migrated runtime mutation path still behaves truthfully

### GFC-11 Runtime recovery or upgrade command boundary

One larger runtime recovery, backup, reload, or upgrade-planning seam must move behind a cleaner application or adapter boundary so the greenfield transition expands beyond the first bounded switch-provider lane.

**Status:** Planned for v1.16 Phase 71.

**Acceptance signals:**
- one larger runtime recovery or upgrade-oriented seam is migrated out of command-local orchestration
- existing runtime/operator behavior remains stable from the shipped contract perspective
- verification proves the migrated command path still behaves truthfully

### GFC-12 High-value control route follow-on extraction

One more bounded `/control/...` route family must stop coupling directly to mixed command-local orchestration and instead call the new greenfield services introduced by the milestone.

**Status:** Planned for v1.16 Phase 72.

**Acceptance signals:**
- one real bounded route family moves behind a stable service boundary
- route behavior remains stable from the runtime API perspective
- the migration materially reduces remaining legacy cross-calls around the newly extracted runtime or skills seams

## Most Recent Archive

- Last shipped milestone: `v1.15 Deeper Greenfield Conversion`
- Archived requirements: `.planning/milestones/v1.15-REQUIREMENTS.md`
- Archived verification bundle: `.planning/milestones/v1.15-VERIFICATIONS.md`

## Next Step

Continue with `$gsd-discuss-phase 69`, `$gsd-plan-phase 69`, or `$gsd-autonomous`.
