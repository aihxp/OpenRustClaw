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
- 🚧 **v1.16 Greenfield Conversion: Skills and Runtime Hotspots** — active

## Current Status

- Active milestone: **v1.16 Greenfield Conversion: Skills and Runtime Hotspots**
- Progress: 0 of 4 phases complete
- Most recent shipment: **v1.15 Deeper Greenfield Conversion**
- Current execution: **Milestone defined; ready for Phase 69**
- Next step: `$gsd-discuss-phase 69`, `$gsd-plan-phase 69`, or `$gsd-autonomous`

## Live Planning

### Phase Checklist

- [ ] **Phase 69: Remaining Skills Plugin Lifecycle Boundary**
- [ ] **Phase 70: Runtime Vault and Secret Mutation Boundary**
- [ ] **Phase 71: Runtime Recovery and Upgrade Boundary**
- [ ] **Phase 72: High-Value Control Route Follow-On Extraction**

### Phase 69: Remaining Skills Plugin Lifecycle Boundary

**Goal:** Move the next remaining plugin-binding, auth-plugin, or voice-plugin mutation lane out of `skills.rs` so the hotspot keeps shrinking after the first registry-mutation extraction.

**Success criteria:**
- one real remaining plugin-lifecycle mutation lane is composed through `openrustclaw-app`
- `skills.rs` becomes the adapter for the migrated plugin lane instead of owning the business rules directly
- verification proves the shipped CLI and control/runtime mutation contract still behaves truthfully

**Plans:** 0/1 plans complete

Plans:
- [ ] 69-01 Extract the next remaining plugin-lifecycle mutation lane from `skills.rs`

### Phase 70: Runtime Vault and Secret Mutation Boundary

**Goal:** Move one real runtime vault, secret, or equivalent configuration-mutation seam into the greenfield lane so `runtime.rs` stops owning more high-risk mutation logic directly.

**Success criteria:**
- one real runtime vault or secret mutation seam is built by `openrustclaw-app`
- the shipped CLI and control API contract remains intact
- verification proves the migrated runtime mutation path still behaves truthfully

**Plans:** 0/1 plans complete

Plans:
- [ ] 70-01 Extract the runtime vault or secret mutation lane

### Phase 71: Runtime Recovery and Upgrade Boundary

**Goal:** Migrate one larger runtime recovery, backup, reload, or upgrade-planning seam behind a cleaner application service so the greenfield transition expands beyond the first bounded provider-switch lane.

**Success criteria:**
- one larger runtime recovery or upgrade-oriented seam moves behind a stable service boundary
- operator-facing runtime behavior remains stable from the shipped contract perspective
- verification proves the migrated command path still behaves truthfully

**Plans:** 0/1 plans complete

Plans:
- [ ] 71-01 Extract one larger runtime recovery or upgrade seam

### Phase 72: High-Value Control Route Follow-On Extraction

**Goal:** Move one more bounded `/control/...` route family behind the new greenfield services introduced by this milestone if it materially reduces remaining legacy coupling.

**Success criteria:**
- one real bounded control route family moves behind a stable application seam
- the runtime API contract remains intact
- the extraction clearly reduces remaining cross-calls around the migrated runtime or skills surfaces

**Plans:** 0/1 plans complete

Plans:
- [ ] 72-01 Extract one high-value follow-on control route family
