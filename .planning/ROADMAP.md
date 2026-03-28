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
- 🚧 **v1.22 Full Greenfield Conversion: Orchestration and Browser Services** — active. Baseline: `3/6` shipped milestones, or about `50%`; target after shipment: `4/6`, or about `67%`

## Current Status

- Active milestone: **v1.22 Full Greenfield Conversion: Orchestration and Browser Services**
- Progress: **0 of 4 phases complete**
- Most recent shipment: **v1.21 Full Greenfield Conversion: Mobile and Voice Runtime Services**
- Greenfield conversion baseline: **historical `18/18` ranked seam ledger complete and retired**
- Full-conversion roadmap progress: **`3/6` milestones shipped, or about `50%`**
- Current execution: **Defining requirements for Phase 93**
- Next step: `$gsd-plan-phase 93` or `$gsd-autonomous`

## Live Planning

### Phase Checklist

- [ ] **Phase 93: Orchestration Request Routing, Override Validation, and Lifecycle-State Transition Services**
- [ ] **Phase 94: Orchestration Checkpoint, Transcript, Trace, Reflection, and Supervision Summary Services**
- [ ] **Phase 95: Browser Backend Policy and Audit Services**
- [ ] **Phase 96: Browser Session Persistence, Workflow Execution, Inspection, and Sequence-Orchestration Services**

### Current Queue Rule

The original ranked greenfield seam inventory remains closed at `18/18` and retired. Follow-on work proceeds under the six-milestone full-conversion roadmap in `.planning/codebase/GREENFIELD-FULL-CONVERSION.md`, which measures progress by migrated adapter-only queues instead of extending the retired historical denominator.

### Phase 93: Orchestration Request Routing, Override Validation, and Lifecycle-State Transition Services

**Goal:** Move orchestration request routing, override validation, and lifecycle-state transition business logic out of `orchestrate.rs` so those flows compose through `openrustclaw-app` instead of route-local command orchestration.

**Depends on:** Phase 92
**Requirements:** `GFC-33`

**Success criteria:**
1. orchestration request routing and override validation compose through `openrustclaw-app`
2. lifecycle-state transition decisions no longer live primarily in `orchestrate.rs`
3. verification proves the shipped orchestration mutation contract remains truthful

**Plans:** 0/1 plans complete

Plans:
- [ ] 93-01 Extract orchestration request routing and lifecycle-state transition services

### Phase 94: Orchestration Checkpoint, Transcript, Trace, Reflection, and Supervision Summary Services

**Goal:** Move the remaining orchestration checkpoint, transcript, trace, reflection, and supervision summary composition behind `openrustclaw-app` so `orchestrate.rs` keeps shrinking toward an adapter-only surface.

**Depends on:** Phase 93
**Requirements:** `GFC-34`

**Success criteria:**
1. orchestration summary and evidence composition flows through `openrustclaw-app`
2. `orchestrate.rs` stops owning the dominant checkpoint, transcript, trace, reflection, and supervision reporting rules
3. verification proves the shipped orchestration reporting contract remains truthful

**Plans:** 0/1 plans complete

Plans:
- [ ] 94-01 Extract orchestration reporting and supervision summary services

### Phase 95: Browser Backend Policy and Audit Services

**Goal:** Move browser backend policy and audit handling behind `openrustclaw-app` so browser control decisions stop deepening legacy command ownership.

**Depends on:** Phase 94
**Requirements:** `GFC-35`

**Success criteria:**
1. browser backend policy and audit flows compose through `openrustclaw-app`
2. legacy browser command surfaces become adapters for policy and audit handling instead of owning business rules directly
3. verification proves the shipped browser policy and audit contract remains truthful

**Plans:** 0/1 plans complete

Plans:
- [ ] 95-01 Extract browser backend policy and audit services

### Phase 96: Browser Session Persistence, Workflow Execution, Inspection, and Sequence-Orchestration Services

**Goal:** Move browser session persistence, workflow execution, inspection, and sequence orchestration behind `openrustclaw-app` so the browser command surface approaches adapter-only ownership for its remaining execution lanes.

**Depends on:** Phase 95
**Requirements:** `GFC-36`

**Success criteria:**
1. browser session persistence and workflow execution compose through `openrustclaw-app`
2. browser inspection and sequence orchestration stop depending on dominant route-local business logic
3. verification proves the shipped browser execution and inspection contract remains truthful

**Plans:** 0/1 plans complete

Plans:
- [ ] 96-01 Extract browser workflow execution and inspection services
