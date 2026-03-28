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
- 🚧 **v1.17 Greenfield Conversion: Completion Metrics and Remaining Hotspots** — active

## Current Status

- Active milestone: **v1.17 Greenfield Conversion: Completion Metrics and Remaining Hotspots**
- Progress: 0 of 4 phases complete
- Most recent shipment: **v1.16 Greenfield Conversion: Skills and Runtime Hotspots**
- Greenfield conversion baseline: **12 of 18 ranked seams migrated (~67%)**
- Current execution: **Milestone defined; ready for Phase 73**
- Next step: `$gsd-plan-phase 73` or `$gsd-autonomous`

## Live Planning

### Phase Checklist

- [ ] **Phase 73: Greenfield Completion Baseline** (not started)
- [ ] **Phase 74: Remaining Skills Auth and Channel Lifecycle Boundary** (not started)
- [ ] **Phase 75: Runtime Upgrade and Rollback Planning Boundary** (not started)
- [ ] **Phase 76: Greenfield Progress Surface and Runtime Maintenance Route** (not started)

### Phase 73: Greenfield Completion Baseline

**Goal:** Define the canonical ranked seam inventory for the brownfield-to-greenfield transition and derive a truthful completion percentage from that inventory so future progress updates stop relying on vague milestone counts.

**Success criteria:**
- one explicit ranked seam inventory exists for the prioritized legacy hotspots
- the current greenfield completion percentage is derived from that inventory and recorded truthfully
- the inventory preserves a clear remaining queue for follow-on conversion work

**Plans:** 0/1 plans complete

Plans:
- [ ] 73-01 Define the seam inventory and percentage baseline

### Phase 74: Remaining Skills Auth and Channel Lifecycle Boundary

**Goal:** Move one real auth-plugin or channel-extension lifecycle lane out of `skills.rs` so the remaining skills hotspot continues shrinking after the compiled-skill, install/update/uninstall, and voice-plugin binding extractions.

**Success criteria:**
- one real auth-plugin or channel-extension lifecycle lane is composed through `openrustclaw-app`
- `skills.rs` becomes the adapter for the migrated lifecycle lane instead of owning the business rules directly
- verification proves the shipped CLI and control/runtime lifecycle contract still behaves truthfully

**Plans:** 0/1 plans complete

Plans:
- [ ] 74-01 Extract one remaining skills auth or channel lifecycle lane

### Phase 75: Runtime Upgrade and Rollback Planning Boundary

**Goal:** Migrate one larger runtime upgrade, self-update, or rollback planning seam behind a cleaner application service so `runtime.rs` continues shrinking beyond provider switching, vault mutation, and reload planning.

**Success criteria:**
- one larger runtime upgrade or rollback-oriented seam moves behind a stable service boundary
- operator-facing runtime behavior remains stable from the shipped contract perspective
- verification proves the migrated planning path still behaves truthfully

**Plans:** 0/1 plans complete

Plans:
- [ ] 75-01 Extract one larger runtime upgrade or rollback planning seam

### Phase 76: Greenfield Progress Surface and Runtime Maintenance Route

**Goal:** Expose the greenfield completion percentage and remaining ranked queue through one shipped inspect or `/control/...` surface while moving one more bounded runtime-maintenance family behind the new application services introduced by this milestone.

**Success criteria:**
- one shipped inspect or control-plane surface reports the greenfield completion percentage and remaining queue
- one real bounded runtime-maintenance route or summary family moves behind a stable application seam
- the extraction clearly reduces remaining route-local or summary-local coupling around runtime maintenance and conversion progress reporting

**Plans:** 0/1 plans complete

Plans:
- [ ] 76-01 Extract the progress-reporting surface and one runtime-maintenance follow-on family
