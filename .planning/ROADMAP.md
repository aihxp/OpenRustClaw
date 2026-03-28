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
- 🚧 **v1.13 Brownfield-to-Greenfield Transition** — active

## Current Status

- Active milestone: **v1.13 Brownfield-to-Greenfield Transition**
- Progress: 4 of 4 phases complete
- Most recent shipment: **v1.12 Secure Node Connectivity and SSH Tunnel Revisit**
- Current execution: **All phases complete; ready for milestone audit**
- Next step: `$gsd-audit-milestone`, `$gsd-complete-milestone`, or `$gsd-autonomous`

## Live Planning

### Phase Checklist

- [x] **Phase 57: Greenfield Boundary Contract and Migration Inventory** (completed 2026-03-28)
- [x] **Phase 58: Greenfield Core Shell and Service Interfaces** (completed 2026-03-28)
- [x] **Phase 59: First Vertical Slice Migration** (completed 2026-03-28)
- [x] **Phase 60: Brownfield Containment and Contributor Defaults** (completed 2026-03-28)

### Phase 57: Greenfield Boundary Contract and Migration Inventory

**Goal:** Define the clean architecture contract, ownership rules, no-touch seams, and first migration targets that turn the repo from opportunistic brownfield change into deliberate greenfield transition work.

**Success criteria:**
- the repo has one canonical greenfield boundary contract
- first migration candidates are chosen explicitly
- brownfield containment rules are documented for future work

**Plans:** 1/1 plans complete

Plans:
- [x] 57-01 Define the architecture boundary and migration inventory

### Phase 58: Greenfield Core Shell and Service Interfaces

**Goal:** Introduce a clean core shell and stable service interfaces so future work can land in the new lane instead of attaching directly to legacy command and runtime modules.

**Success criteria:**
- a real greenfield core shell exists in shipped code
- dependency direction is cleaner and bounded
- stable service interfaces exist for the first migration work

**Plans:** 1/1 plans complete

Plans:
- [x] 58-01 Create the greenfield core shell and service boundaries

### Phase 59: First Vertical Slice Migration

**Goal:** Migrate one high-value shipped vertical slice into the new architecture lane to prove the transition with production code.

**Success criteria:**
- one real operator or user flow runs through the new boundary
- direct coupling to legacy modules is reduced for that slice
- verification preserves end-to-end behavior for the migrated path

**Plans:** 1/1 plans complete

Plans:
- [x] 59-01 Migrate the first proving slice into the greenfield lane

### Phase 60: Brownfield Containment and Contributor Defaults

**Goal:** Make the new lane the default for future work through contributor guidance, compatibility rules, and explicit deprecation follow-up.

**Success criteria:**
- contributor guidance points new work at the new lane
- compatibility rules are explicit for mixed old and new surfaces
- deprecation or follow-on migration work is preserved truthfully

**Plans:** 1/1 plans complete

Plans:
- [x] 60-01 Lock contributor defaults and brownfield containment rules
