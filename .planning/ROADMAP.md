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
- ✅ **v1.23 Full Greenfield Conversion: Setup and Secondary Command Surfaces** — shipped 2026-03-28. Archive: `.planning/milestones/v1.23-ROADMAP.md`
- 🚧 **v1.24 Full Greenfield Conversion: Adapter-Only Exit and Enforcement** — active. Baseline: `5/6` shipped milestones, or about `83%`; target after shipment: `6/6`, or `100%`

## Current Status

- Active milestone: **v1.24 Full Greenfield Conversion: Adapter-Only Exit and Enforcement**
- Progress: **0 of 4 phases complete**
- Most recent shipment: **v1.23 Full Greenfield Conversion: Setup and Secondary Command Surfaces**
- Greenfield conversion baseline: **historical `18/18` ranked seam ledger complete and retired**
- Full-conversion roadmap progress: **`5/6` milestones shipped, or about `83%`**
- Current execution: **Phase 101 ready for planning**
- Next step: `$gsd-plan-phase 101` or `$gsd-autonomous`

## Live Planning

### Phase Checklist

- [ ] **Phase 101: Final Residual Helper Extraction and Hotspot Deletion**
- [ ] **Phase 102: Adapter and Port Boundary Formalization**
- [ ] **Phase 103: Architecture Guardrails and Contribution Enforcement**
- [ ] **Phase 104: Full-Conversion Exit Audit and Scorecard**

### Current Queue Rule

The original ranked greenfield seam inventory remains closed at `18/18` and retired. Follow-on work proceeds under the six-milestone full-conversion roadmap in `.planning/codebase/GREENFIELD-FULL-CONVERSION.md`, which measures progress by migrated adapter-only queues instead of extending the retired historical denominator.

### Phase 101: Final Residual Helper Extraction and Hotspot Deletion

**Goal:** Remove or extract the last remaining mixed-responsibility helper ownership from the targeted legacy command hotspots so the final queue starts by shrinking real adapter drift instead of adding new abstraction labels.

**Depends on:** Phase 100
**Requirements:** `GFC-41`

**Success criteria:**
1. the targeted final residual helper seams are either extracted behind `openrustclaw-app` or deleted
2. the affected legacy command modules lose more hidden business-rule ownership instead of gaining new local helpers
3. verification proves the migrated operator and runtime contracts remain truthful after the cleanup

**Plans:** 0/0 plans complete

Plans:
- none yet

### Phase 102: Adapter and Port Boundary Formalization

**Goal:** Make the remaining persistence and external side-effect boundaries explicit so the final legacy command surfaces read as adapters over named ports instead of mixed orchestration hubs.

**Depends on:** Phase 101
**Requirements:** `GFC-42`

**Success criteria:**
1. the targeted persistence and integration seams use explicit adapter or port boundaries
2. remaining legacy command modules read primarily as transport, workspace, or external-system adapters
3. verification proves the extracted boundaries preserve the shipped command and control contracts

**Plans:** 0/0 plans complete

Plans:
- none yet

### Phase 103: Architecture Guardrails and Contribution Enforcement

**Goal:** Add durable architecture guardrails so new business logic does not silently drift back into legacy command hubs after the full-conversion work ships.

**Depends on:** Phase 102
**Requirements:** `GFC-43`

**Success criteria:**
1. the repo contains explicit guardrails that block or warn on new business logic landing in legacy command hotspots
2. contributor-facing planning and architecture surfaces point new logic to the greenfield lane by default
3. verification proves the enforcement layer itself is stable and maintainable

**Plans:** 0/0 plans complete

Plans:
- none yet

### Phase 104: Full-Conversion Exit Audit and Scorecard

**Goal:** Close the six-milestone full-conversion program with a truthful audit, verification bundle, and exit scorecard that states whether the adapter-only architecture claim is now warranted.

**Depends on:** Phase 103
**Requirements:** `GFC-44`

**Success criteria:**
1. the final audit and verification bundle cover the full-conversion exit criteria explicitly
2. the shipped planning and contributor surfaces report the broader roadmap as `6/6`, or `100%`, only if the exit criteria are met
3. any remaining exceptions are documented truthfully instead of being hidden behind a blanket completion claim

**Plans:** 0/0 plans complete

Plans:
- none yet
