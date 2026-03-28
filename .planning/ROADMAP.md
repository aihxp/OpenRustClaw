# Roadmap: OpenRustClaw

## Milestones

- ✅ **v1.0 Rust OpenClaw MVP** — shipped 2026-03-26. Archive: `.planning/milestones/v1.0-ROADMAP.md`
- ✅ **v1.1 Lifecycle Integrity and Enterprise Foundations** — shipped 2026-03-26. Archive: `.planning/milestones/v1.1-ROADMAP.md`
- ✅ **v1.2 Deeper OpenClaw Surface Parity** — shipped 2026-03-27. Archive: `.planning/milestones/v1.2-ROADMAP.md`
- ✅ **v1.3 Enterprise Expansion and Supervised Autonomy Foundations** — shipped 2026-03-27. Archive: `.planning/milestones/v1.3-ROADMAP.md`
- ✅ **v1.4 Enterprise Governance and Operator-Gated Full Autonomy** — shipped 2026-03-27. Archive: `.planning/milestones/v1.4-ROADMAP.md`
- ✅ **v1.5 Self-Hosted Product Modes and Lifecycle Packaging** — shipped 2026-03-27. Archive: `.planning/milestones/v1.5-ROADMAP.md`
- ✅ **v1.6 Proper Onboarding and Setup** — shipped 2026-03-28. Archive: `.planning/milestones/v1.6-ROADMAP.md`
- ✅ **v1.7 Documentation Convergence and OpenClaw-Inspired Docs Rewrite** — shipped 2026-03-27. Archive: `.planning/milestones/v1.7-ROADMAP.md`
- ✅ **v1.8 Clean Codebase** — shipped 2026-03-27. Archive: `.planning/milestones/v1.8-ROADMAP.md`
- ✅ **v1.9 GitHub Repository Presence and Actions Recovery** — shipped 2026-03-28. Archive: `.planning/milestones/v1.9-ROADMAP.md`
- ✅ **v1.10 Release Binaries Workflow Recovery** — shipped 2026-03-28. Archive: `.planning/milestones/v1.10-ROADMAP.md`
- ✅ **v1.11 Crates.io and Docs.rs Publication Foundation** — shipped 2026-03-28. Archive: `.planning/milestones/v1.11-ROADMAP.md`
- ✅ **v1.12 Secure Node Connectivity and SSH Tunnel Revisit** — shipped 2026-03-28. Archive: `.planning/milestones/v1.12-ROADMAP.md`
- ✅ **v1.13 Brownfield-to-Greenfield Transition** — shipped 2026-03-28. Archive: `.planning/milestones/v1.13-ROADMAP.md`
- ✅ **v1.14 Continued Greenfield Conversion** — shipped 2026-03-28. Archive: `.planning/milestones/v1.14-ROADMAP.md`
- ✅ **v1.15 Deeper Greenfield Conversion** — shipped 2026-03-28. Archive: `.planning/milestones/v1.15-ROADMAP.md`
- ✅ **v1.16 Greenfield Conversion: Skills and Runtime Hotspots** — shipped 2026-03-28. Archive: `.planning/milestones/v1.16-ROADMAP.md`
- ✅ **v1.17 Greenfield Conversion: Completion Metrics and Remaining Hotspots** — shipped 2026-03-28. Archive: `.planning/milestones/v1.17-ROADMAP.md`
- ✅ **v1.18 Greenfield Conversion: Final Ranked Seam and 100% Completion Path** — shipped 2026-03-28. Archive: `.planning/milestones/v1.18-ROADMAP.md`
- ✅ **v1.19 Full Greenfield Conversion: Control Plane Route Families I** — shipped 2026-03-28. Archive: `.planning/milestones/v1.19-ROADMAP.md`
- 🚧 **v1.20 Full Greenfield Conversion: Control Plane Route Families II** — active

## Current Status

- Active milestone: **v1.20 Full Greenfield Conversion: Control Plane Route Families II**
- Progress: **0 of 4 phases complete**
- Most recent shipment: **v1.19 Full Greenfield Conversion: Control Plane Route Families I**
- Greenfield conversion baseline: **historical `18/18` ranked seam ledger complete; deeper follow-on program now begins with remaining `start.rs` route families**
- Remaining ranked seam: **the retired `18/18` ledger remains closed; follow-on work now uses the full-conversion roadmap in `.planning/codebase/GREENFIELD-FULL-CONVERSION.md`**
- Full-conversion roadmap progress: **`1/6` milestones shipped, or about `17%`; `v1.20` targets `2/6`, or about `33%`**
- Current execution: **Second follow-on control-plane route queue defined**
- Next step: `$gsd-plan-phase 85` or `$gsd-autonomous`

## Live Planning

### Phase Checklist

- [ ] **Phase 85: Autonomy Lessons and Lesson Mutation Route Families**
- [ ] **Phase 86: Remaining Skill-Control Route Families**
- [ ] **Phase 87: Voice-Call and Channel-Extension Control Route Families**
- [ ] **Phase 88: Control-Plane Route Registration and Shared State Cleanup**

### Current Queue Rule

The original ranked greenfield seam inventory is closed at `18/18` and retired. Follow-on work now proceeds under the full-conversion roadmap in `.planning/codebase/GREENFIELD-FULL-CONVERSION.md`, which aims for adapter-only legacy command surfaces instead of extending the retired historical denominator.

### Phase 85: Autonomy Lessons and Lesson Mutation Route Families

**Goal:** Move the autonomy lesson listing, detail, and mutation route families out of `start.rs` so lesson-control flows become another bounded application-owned route family.

**Success criteria:**
- the autonomy lesson route family is composed through `openrustclaw-app`
- `start.rs` becomes the HTTP adapter for lesson summary and lesson mutation flows instead of owning the business rules directly
- verification proves the shipped lesson-control contract remains truthful

**Plans:** 0/0 plans complete

### Phase 86: Remaining Skill-Control Route Families

**Goal:** Move the remaining skill-control route families out of `start.rs` so those control surfaces stop relying on route-local orchestration over legacy command helpers.

**Success criteria:**
- the remaining skill-control route families are composed through `openrustclaw-app`
- `start.rs` becomes the HTTP adapter for those skill-control surfaces instead of owning orchestration directly
- verification proves the shipped skill-control contract remains truthful

**Plans:** 0/0 plans complete

### Phase 87: Voice-Call and Channel-Extension Control Route Families

**Goal:** Move the remaining voice-call and channel-extension control route families out of `start.rs` so those control surfaces stop depending on route-local business logic.

**Success criteria:**
- voice-call and channel-extension control routes compose through `openrustclaw-app`
- `start.rs` becomes the HTTP adapter for those control surfaces instead of owning business orchestration directly
- verification proves the shipped voice-call and channel-extension control contract remains truthful

**Plans:** 0/0 plans complete

### Phase 88: Control-Plane Route Registration and Shared State Cleanup

**Goal:** Reduce shared route registration, state wiring, and helper sprawl after the first two control-plane route milestones so future route extractions stop depending on oversized `start.rs` setup.

**Success criteria:**
- shared control-plane route registration and state wiring are materially simpler after the migrated route families
- the migrated route families no longer require ad hoc `start.rs` helper sprawl to register or resolve shared state
- verification proves the shipped control-plane route map still behaves truthfully after cleanup

**Plans:** 0/0 plans complete
