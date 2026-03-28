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
- ✅ **v1.20 Full Greenfield Conversion: Control Plane Route Families II** — shipped 2026-03-28. Archive: `.planning/milestones/v1.20-ROADMAP.md`
- ✅ **v1.21 Full Greenfield Conversion: Mobile and Voice Runtime Services** — shipped 2026-03-28. Archive: `.planning/milestones/v1.21-ROADMAP.md`
- ✅ **v1.22 Full Greenfield Conversion: Orchestration and Browser Services** — shipped 2026-03-28. Archive: `.planning/milestones/v1.22-ROADMAP.md`
- 🚧 **v1.23 Full Greenfield Conversion: Setup and Secondary Command Surfaces** — active. Baseline: `4/6` shipped milestones, or about `67%`; target after shipment: `5/6`, or about `83%`

## Current Status

- Active milestone: **v1.23 Full Greenfield Conversion: Setup and Secondary Command Surfaces**
- Progress: **0 of 4 phases complete**
- Most recent shipment: **v1.22 Full Greenfield Conversion: Orchestration and Browser Services**
- Greenfield conversion baseline: **historical `18/18` ranked seam ledger complete and retired**
- Full-conversion roadmap progress: **`4/6` milestones shipped, or about `67%`**
- Current execution: **Phase 97 ready for planning**
- Next step: `$gsd-plan-phase 97` or `$gsd-autonomous`

## Live Planning

### Phase Checklist

- [ ] **Phase 97: Onboarding, Repair, and Resume Orchestration Services**
- [ ] **Phase 98: Secondary Lifecycle Command Services**
- [ ] **Phase 99: Secondary Operator, Media, Tools, and Memory Service Seams**
- [ ] **Phase 100: Transition Helper Cleanup and Adapter Convergence**

### Current Queue Rule

The original ranked greenfield seam inventory remains closed at `18/18` and retired. Follow-on work proceeds under the six-milestone full-conversion roadmap in `.planning/codebase/GREENFIELD-FULL-CONVERSION.md`, which measures progress by migrated adapter-only queues instead of extending the retired historical denominator.

### Phase 97: Onboarding, Repair, and Resume Orchestration Services

**Goal:** Move onboarding, repair, and resume orchestration business logic out of `onboard.rs` so setup lifecycle flows compose through `openrustclaw-app` instead of command-local orchestration.

**Depends on:** Phase 96
**Requirements:** `GFC-37`

**Success criteria:**
1. onboarding, repair, and resume orchestration compose through `openrustclaw-app`
2. `onboard.rs` no longer owns the dominant setup-transition and step-planning rules for the targeted slice
3. verification proves the shipped setup lifecycle contract remains truthful

**Plans:** 0/0 plans complete

Plans:
- none yet

### Phase 98: Secondary Lifecycle Command Services

**Goal:** Move the targeted residual lifecycle seams in `channels.rs`, `schedule.rs`, `services.rs`, and `control.rs` behind `openrustclaw-app` so those command modules keep shrinking toward adapter-only ownership.

**Depends on:** Phase 97
**Requirements:** `GFC-38`

**Success criteria:**
1. the targeted secondary lifecycle command seams compose through `openrustclaw-app`
2. legacy lifecycle command modules stop owning the dominant mutation and report-composition rules for those seams
3. verification proves the shipped lifecycle and control contracts remain truthful

**Plans:** 0/0 plans complete

Plans:
- none yet

### Phase 99: Secondary Operator, Media, Tools, and Memory Service Seams

**Goal:** Move the targeted residual operator, media, tools, and memory helper seams behind `openrustclaw-app` so neighboring secondary command modules stop deepening brownfield ownership.

**Depends on:** Phase 98
**Requirements:** `GFC-39`

**Success criteria:**
1. the targeted operator, media, tools, and memory seams compose through `openrustclaw-app`
2. affected secondary command modules become adapters around bounded workspace, runtime, or artifact I/O
3. verification proves the shipped operator-facing helper contracts remain truthful

**Plans:** 0/0 plans complete

Plans:
- none yet

### Phase 100: Transition Helper Cleanup and Adapter Convergence

**Goal:** Clean up transition-era helper duplication and normalize the affected adapters after the setup and secondary command-surface extractions land.

**Depends on:** Phase 99
**Requirements:** `GFC-40`

**Success criteria:**
1. duplicated transition-era helper logic is removed, consolidated, or explicitly bounded
2. the affected command modules expose clearer shared adapter and service boundaries after the milestone extractions
3. verification proves the cleanup does not regress the migrated setup and secondary command contracts

**Plans:** 0/0 plans complete

Plans:
- none yet
