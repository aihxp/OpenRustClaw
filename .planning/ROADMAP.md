# Roadmap: OpenRustClaw

## Milestones

- ✅ **v1.0 Rust OpenClaw MVP** — shipped 2026-03-26. Archive: `.planning/milestones/v1.0-ROADMAP.md`
- ✅ **v1.1 Lifecycle Integrity and Enterprise Foundations** — shipped 2026-03-26. Archive: `.planning/milestones/v1.1-ROADMAP.md`
- ✅ **v1.2 Deeper OpenClaw Surface Parity** — shipped 2026-03-27. Archive: `.planning/milestones/v1.2-ROADMAP.md`
- ✅ **v1.3 Enterprise Expansion and Supervised Autonomy Foundations** — shipped 2026-03-27. Archive: `.planning/milestones/v1.3-ROADMAP.md`
- ✅ **v1.4 Enterprise Governance and Operator-Gated Full Autonomy** — shipped 2026-03-27. Archive: `.planning/milestones/v1.4-ROADMAP.md`
- ✅ **v1.5 Self-Hosted Product Modes and Lifecycle Packaging** — shipped 2026-03-27. Archive: `.planning/milestones/v1.5-ROADMAP.md`
- 🚧 **v1.6 Proper Onboarding and Setup** — phases 28-31

## Roadmap v1.6: Proper Onboarding and Setup

### Overview

This milestone turns onboarding and setup into one believable self-hosted product journey. The order is deliberate: first define durable setup state plus a standard-versus-advanced setup-path contract, then make provider or runtime or channel bootstrap mode-aware, then harden repair and re-entry for partial installs, and finally close with a truthful setup handoff and aligned operator surfaces.

### Phases

**Phase Numbering:**
- Integer phases continue across milestones to preserve one linear execution history.
- Decimal phases are reserved for urgent insertions if roadmap assumptions break.

- [x] **Phase 28: Setup State and Resumable Onboarding Contract** - completed 2026-03-28. Defined the durable setup-state model and resumable onboarding flow.
- [x] **Phase 29: Mode-Aware Provider, Runtime, and Channel Bootstrap** - completed 2026-03-28. Setup now validates provider, runtime, and channel bootstrap through shipped health/probe surfaces and persists those outcomes in setup state.
- [ ] **Phase 30: Setup Repair and Existing Workspace Recovery** - Add explicit resume, repair, and reset-with-backup paths for partial or drifted setups.
- [ ] **Phase 31: Setup Handoff and Operator Surface Alignment** - Close the loop with an explicit setup summary, next-action handoff, and aligned docs or control surfaces.

### Phase Details

### Phase 28: Setup State and Resumable Onboarding Contract
**Goal**: Define the durable setup-state contract so onboarding can resume truthfully instead of acting like every run is a fresh workspace.
**Depends on**: v1.5 archive state
**Requirements**: [SETUP-01, PATH-01]
**Success Criteria** (what must be TRUE):
  1. Setup progress is persisted as first-class state rather than inferred from scattered files.
  2. The chosen deployment mode, setup depth, completed steps, blockers, and next action are inspectable.
  3. Operators can choose a standard path or an advanced/custom path without creating a disconnected setup state model.
**Plans**: 2 plans complete

Plans:
- [x] 28-01 Add Durable Setup-State Contract
- [x] 28-02 Resume Onboarding with Standard, Advanced, and Custom Paths

### Phase 29: Mode-Aware Provider, Runtime, and Channel Bootstrap
**Goal**: Make setup actually configure and validate the core runtime path for the selected deployment mode.
**Depends on**: Phase 28
**Requirements**: [BOOT-01, BOOT-02, PATH-02]
**Success Criteria** (what must be TRUE):
  1. Setup drives provider, model, runtime, and control-plane configuration to a truthful ready or blocked state.
  2. Setup can validate and, where supported, bootstrap key runtime or channel surfaces before claiming success.
  3. Standard and advanced/custom setup paths remain mode-aware but converge on the same readiness contract.
**Plans**: 3 plans complete

Plans:
- [x] 29-01 Persist Bootstrap Outcomes in Setup State
- [x] 29-02 Validate Provider and Runtime Bootstrap During Onboarding
- [x] 29-03 Validate Channel Bootstrap During Onboarding

### Phase 30: Setup Repair and Existing Workspace Recovery
**Goal**: Give operators explicit ways to resume, repair, or reset partial setups without manual workspace surgery.
**Depends on**: Phase 29
**Requirements**: [SETUP-02]
**Success Criteria** (what must be TRUE):
  1. Existing or partial workspaces can re-enter setup through explicit choices instead of hidden heuristics.
  2. Repair and reset-with-backup flows preserve trust and visibility around what will change.
  3. Setup recovery uses the same durable setup-state contract instead of bypassing it.
**Plans**: TBD

Plans:
- [ ] TBD (run `$gsd-plan-phase 30` to break down)

### Phase 31: Setup Handoff and Operator Surface Alignment
**Goal**: End setup with a clear operator handoff and make the same setup state legible across docs and shipped surfaces.
**Depends on**: Phase 30
**Requirements**: [HANDOFF-01, HANDOFF-02]
**Success Criteria** (what must be TRUE):
  1. Setup ends with explicit ready, blocked, or degraded status plus concrete next actions.
  2. The operator-facing setup story is consistent across onboarding output, docs, and shipped dashboard surfaces.
  3. The milestone closes with a truthful setup baseline rather than another partial wizard improvement.
**Plans**: TBD

Plans:
- [ ] TBD (run `$gsd-plan-phase 31` to break down)

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 28. Setup State and Resumable Onboarding Contract | 2/2 | Complete | 2026-03-28 |
| 29. Mode-Aware Provider, Runtime, and Channel Bootstrap | 3/3 | Complete | 2026-03-28 |
| 30. Setup Repair and Existing Workspace Recovery | 0/TBD | Not started | - |
| 31. Setup Handoff and Operator Surface Alignment | 0/TBD | Not started | - |

## Current Status

- Active milestone: v1.6 Proper Onboarding and Setup
- Next step: `$gsd-discuss-phase 30` or `$gsd-plan-phase 30`
- Archived milestone planning artifacts live under `.planning/milestones/`.

## Phase History

<details>
<summary>✅ v1.5 Self-Hosted Product Modes and Lifecycle Packaging — SHIPPED 2026-03-27</summary>

- [x] **Phase 24: Self-Hosted Product Modes and Instance Profile Contract** — completed 2026-03-27
- [x] **Phase 25: Tiered Onboarding and First-Run Paths** — completed 2026-03-27
- [x] **Phase 26: Upgrade and Downgrade Lifecycle** — completed 2026-03-27
- [x] **Phase 27: Self-Hosted Product Surface Alignment** — completed 2026-03-27

</details>

<details>
<summary>✅ v1.4 Enterprise Governance and Operator-Gated Full Autonomy — SHIPPED 2026-03-27</summary>

- [x] **Phase 20: Enterprise Governance and Approval Chains** — completed 2026-03-27
- [x] **Phase 21: Enterprise Audit Retention and Review Packaging** — completed 2026-03-27
- [x] **Phase 22: Operator-Gated Full Autonomy Mode** — completed 2026-03-27
- [x] **Phase 23: Enterprise Autonomy Control Surface** — completed 2026-03-27

</details>

<details>
<summary>✅ v1.3 Enterprise Expansion and Supervised Autonomy Foundations — SHIPPED 2026-03-27</summary>

- [x] **Phase 16: Enterprise Identity and Access Boundaries** — completed 2026-03-27
- [x] **Phase 17: Enterprise Policy and Audit Controls** — completed 2026-03-27
- [x] **Phase 18: Supervised Autonomy Escalation and Rollback** — completed 2026-03-27
- [x] **Phase 19: Enterprise Admin Surface** — completed 2026-03-27

</details>

<details>
<summary>✅ v1.2 Deeper OpenClaw Surface Parity — SHIPPED 2026-03-27</summary>

- [x] **Phase 11: Browser Automation Depth** — completed 2026-03-27
- [x] **Phase 12: Multi-Agent Supervision Parity** — completed 2026-03-27
- [x] **Phase 13: Mobile Runtime Parity** — completed 2026-03-27
- [x] **Phase 14: Control UI Surface Completion** — completed 2026-03-27
- [x] **Phase 15: Voice and Call Handling Parity** — completed 2026-03-27

</details>

<details>
<summary>✅ v1.1 Lifecycle Integrity and Enterprise Foundations — SHIPPED 2026-03-26</summary>

- [x] **Phase 8: Verification Artifact Contract** — completed 2026-03-26
- [x] **Phase 9: Milestone Lifecycle Integrity** — completed 2026-03-26
- [x] **Phase 10: Enterprise Policy and Audit Foundations** — completed 2026-03-26

</details>

<details>
<summary>✅ v1.0 Rust OpenClaw MVP — SHIPPED 2026-03-26</summary>

- [x] **Phase 1: Onboarding and First-Run Trust** — completed 2026-03-26
- [x] **Phase 2: Core Assistant and Session Continuity** — completed 2026-03-26
- [x] **Phase 3: Memory Durability and Write Policy** — completed 2026-03-26
- [x] **Phase 4: Tool, MCP, and Coding Workflow Hardening** — completed 2026-03-26
- [x] **Phase 5: Email and Voice Communications** — completed 2026-03-26
- [x] **Phase 6: Deployment, Runtime, and Operator Ops** — completed 2026-03-26
- [x] **Phase 7: Security, Observability, and Release Exit** — completed 2026-03-26

</details>
