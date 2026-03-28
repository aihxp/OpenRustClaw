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
- 🚧 **v1.14 Continued Greenfield Conversion** — active

## Current Status

- Active milestone: **v1.14 Continued Greenfield Conversion**
- Progress: 3 of 4 phases complete
- Most recent shipment: **v1.13 Brownfield-to-Greenfield Transition**
- Current execution: **Phase 64 not started**
- Next step: `$gsd-discuss-phase 64`, `$gsd-plan-phase 64`, or `$gsd-autonomous`

## Live Planning

### Phase Checklist

- [x] **Phase 61: Inspection Summary Service Extraction** (completed 2026-03-28)
- [x] **Phase 62: Control Route Family Service Extraction** (completed 2026-03-28)
- [x] **Phase 63: Mobile Operator Report Migration** (completed 2026-03-28)
- [ ] **Phase 64: Skills Surface Boundary Cleanup**

### Phase 61: Inspection Summary Service Extraction

**Goal:** Move broader inspection-summary composition into `openrustclaw-app` so `inspect.rs` stops acting as the long-term business-logic owner for typed operator summaries.

**Success criteria:**
- at least one additional typed inspection family is composed in `openrustclaw-app`
- `inspect.rs` becomes an adapter for the migrated summary
- existing report-facing verification remains truthful

**Plans:** 1/1 plans complete

Plans:
- [x] 61-01 Migrate the next inspection summary family into the application lane

### Phase 62: Control Route Family Service Extraction

**Goal:** Reduce direct business-logic ownership inside selected `start.rs` control route families by moving them behind cleaner application-facing services.

**Success criteria:**
- one bounded control route family runs through a cleaner service boundary
- route behavior stays stable from the runtime API perspective
- `start.rs` loses direct mixed-command coupling for the migrated family

**Plans:** 1/1 plans complete

Plans:
- [x] 62-01 Extract a bounded control route family behind an application service seam

### Phase 63: Mobile Operator Report Migration

**Goal:** Prove the greenfield lane on a second operator-visible surface by moving one mobile operator report into `openrustclaw-app`.

**Success criteria:**
- one real mobile operator report is built by the application lane
- runtime and Control UI contracts stay intact
- verification proves the migrated mobile surface still behaves truthfully

**Plans:** 1/1 plans complete

Plans:
- [x] 63-01 Migrate the next mobile operator report into the application lane

### Phase 64: Skills Surface Boundary Cleanup

**Goal:** Create the first bounded service seam for `skills.rs` so future work stops treating that hotspot as the default home for new behavior.

**Success criteria:**
- one meaningful `skills.rs` slice is extracted behind a stable service or adapter boundary
- contributor guidance can point future work at the new seam
- remaining `skills.rs` cleanup debt is preserved explicitly

**Plans:** 0/1 plans complete

Plans:
- [ ] 64-01 Extract the first stable service seam from `skills.rs`
