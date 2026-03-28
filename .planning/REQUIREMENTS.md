# Requirements: OpenRustClaw

## Active Milestone: v1.20 Full Greenfield Conversion: Control Plane Route Families II

**Goal:** Continue the post-`18/18` greenfield program by shrinking the next remaining control-plane hotspots in `start.rs`, focusing on lessons, skill-control, voice-call and channel-extension routes, plus shared route-state cleanup.

**Current greenfield baseline:** The historical ranked seam ledger from `v1.13` through `v1.18` remains complete at `18/18` and retired. The deeper full-conversion roadmap now stands at `1/6` milestones shipped, or about `17%` complete, with `v1.20` targeting `2/6`, or about `33%`, in `.planning/codebase/GREENFIELD-FULL-CONVERSION.md`.

## Requirements

### GFC-25 Autonomy lessons and lesson mutation route families

The autonomy lesson listing, detail, and mutation route families in `crates/cli/src/commands/start.rs` must move behind stable application service boundaries so lesson-control flows stop owning business orchestration inside the route hub.

**Status:** Planned for v1.20 Phase 85.

**Acceptance signals:**
- the autonomy lesson route family is composed through `openrustclaw-app`
- `start.rs` becomes the HTTP adapter for lesson summary and lesson mutation flows instead of owning the business rules directly
- verification proves the shipped lesson-control contract remains truthful

### GFC-26 Remaining skill-control route families

The remaining skill-control route families in `crates/cli/src/commands/start.rs` that still adapt route-local logic over legacy command helpers must move behind stable application boundaries so the control plane stops deepening that legacy orchestration path.

**Status:** Planned for v1.20 Phase 86.

**Acceptance signals:**
- the remaining skill-control route families are composed through `openrustclaw-app`
- `start.rs` becomes the HTTP adapter for those skill-control surfaces instead of owning orchestration directly
- verification proves the shipped skill-control contract remains truthful

### GFC-27 Voice-call and channel-extension control route families

The remaining voice-call and channel-extension control route families in `crates/cli/src/commands/start.rs` must move behind bounded application-owned orchestration so those control surfaces stop depending on route-local business logic.

**Status:** Planned for v1.20 Phase 87.

**Acceptance signals:**
- voice-call and channel-extension control routes compose through `openrustclaw-app`
- `start.rs` becomes the HTTP adapter for those control surfaces instead of owning business orchestration directly
- verification proves the shipped voice-call and channel-extension control contract remains truthful

### GFC-28 Control-plane route registration and shared state cleanup

Shared route registration, state wiring, and cleanup after the first two full-conversion control-plane milestones must be reduced so future route extractions stop depending on oversized `start.rs` setup and ad hoc shared helper drift.

**Status:** Planned for v1.20 Phase 88.

**Acceptance signals:**
- shared control-plane route registration and state wiring are materially simpler after the migrated route families
- the migrated route families no longer require ad hoc `start.rs` helper sprawl to register or resolve shared state
- verification proves the shipped control-plane route map still behaves truthfully after cleanup

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| GFC-25 | Phase 85 | Pending |
| GFC-26 | Phase 86 | Pending |
| GFC-27 | Phase 87 | Pending |
| GFC-28 | Phase 88 | Pending |

**Coverage:**
- v1 requirements: 4 total
- Mapped to phases: 4
- Unmapped: 0

## Most Recent Archive

- Last shipped milestone: `v1.19 Full Greenfield Conversion: Control Plane Route Families I`
- Archived requirements: `.planning/milestones/v1.19-REQUIREMENTS.md`
- Archived verification bundle: `.planning/milestones/v1.19-VERIFICATIONS.md`

## Next Step

Plan and execute `v1.20` starting with `$gsd-plan-phase 85`.
