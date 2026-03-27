# Roadmap: OpenRustClaw

## Milestones

- 🚧 **v1.8 Clean Codebase** — phases 37-40
- ✅ **v1.0 Rust OpenClaw MVP** — shipped 2026-03-26. Archive: `.planning/milestones/v1.0-ROADMAP.md`
- ✅ **v1.1 Lifecycle Integrity and Enterprise Foundations** — shipped 2026-03-26. Archive: `.planning/milestones/v1.1-ROADMAP.md`
- ✅ **v1.2 Deeper OpenClaw Surface Parity** — shipped 2026-03-27. Archive: `.planning/milestones/v1.2-ROADMAP.md`
- ✅ **v1.3 Enterprise Expansion and Supervised Autonomy Foundations** — shipped 2026-03-27. Archive: `.planning/milestones/v1.3-ROADMAP.md`
- ✅ **v1.4 Enterprise Governance and Operator-Gated Full Autonomy** — shipped 2026-03-27. Archive: `.planning/milestones/v1.4-ROADMAP.md`
- ✅ **v1.5 Self-Hosted Product Modes and Lifecycle Packaging** — shipped 2026-03-27. Archive: `.planning/milestones/v1.5-ROADMAP.md`
- ✅ **v1.6 Proper Onboarding and Setup** — shipped 2026-03-28. Archive: `.planning/milestones/v1.6-ROADMAP.md`
- ✅ **v1.7 Documentation Convergence and OpenClaw-Inspired Docs Rewrite** — shipped 2026-03-27. Archive: `.planning/milestones/v1.7-ROADMAP.md`

## Current Status

- Active milestone: v1.8 Clean Codebase
- Next step: `$gsd-discuss-phase 37`
- Archived milestone planning artifacts live under `.planning/milestones/`.

## Roadmap v1.8: Clean Codebase

### Overview

This milestone treats cleanup as a real delivery lane. The order is deliberate: first make cleanup targets and no-touch boundaries explicit so refactors stop being guesswork, then remove stale repo drift and local-noise paths that no longer belong in the canonical product surface, then decompose the highest-risk oversized command or control surfaces behind stable contracts, and finally lock the cleanup in with targeted verification plus a maintained remaining-debt record.

### Phases

**Phase Numbering:**
- Integer phases continue across milestones to preserve one linear execution history.
- Decimal phases are reserved for urgent insertions if roadmap assumptions break.

- [x] **Phase 37: Codebase Cleanup Inventory and Refactor Contract** - completed 2026-03-27. Cleanup targets, no-touch boundaries, and the milestone cleanup order are now explicit.
- [x] **Phase 38: Repo Hygiene and Drift Reduction** - completed 2026-03-27. CI now matches the current canonical docs, and sidecar local Python state is explicitly non-canonical repo surface.
- [ ] **Phase 39: Command Surface Decomposition and Boundary Cleanup** - split the highest-risk oversized command or control surfaces into smaller bounded units without changing shipped behavior.
- [ ] **Phase 40: Cleanup Verification and Maintenance Guardrails** - add targeted verification, document remaining debt, and preserve the cleanup contract for future milestones.

### Phase Details

### Phase 37: Codebase Cleanup Inventory and Refactor Contract
**Goal**: Turn cleanup into an explicit brownfield contract so later refactors are guided by one inventory of priority hotspots, safe boundaries, and no-touch zones.
**Depends on**: v1.7 archive state
**Requirements**: [CLEAN-01, CLEAN-02]
**Success Criteria** (what must be TRUE):
  1. The repo identifies canonical, oversized, deprecated, generated, and cleanup-candidate surfaces in one maintained inventory.
  2. High-risk hotspots and no-touch boundaries are explicit before code movement starts.
  3. The milestone has a prioritized cleanup target list instead of ad hoc refactor guesses.
**Plans**: 2 plans complete

Plans:
- [x] 37-01 Inventory cleanup hotspots and boundaries
- [x] 37-02 Map cleanup order and guarded follow-up slices

### Phase 38: Repo Hygiene and Drift Reduction
**Goal**: Remove stale or drifting repo surfaces so the workspace, CI, and docs all point at the actual shipped product contract.
**Depends on**: Phase 37
**Requirements**: [HYGI-01, HYGI-02]
**Success Criteria** (what must be TRUE):
  1. Stale, duplicate, generated, or local-environment noise paths targeted by the cleanup contract are removed, ignored, or relocated appropriately.
  2. CI, docs, and canonical filenames no longer reference known missing or renamed surfaces.
  3. The repo tree is measurably clearer for maintainers before structural refactors continue.
**Plans**: 2 plans complete

Plans:
- [x] 38-01 Align CI with canonical planning docs
- [x] 38-02 Tighten sidecar hygiene boundaries

### Phase 39: Command Surface Decomposition and Boundary Cleanup
**Goal**: Reduce regression risk by shrinking at least the priority oversized command or control surfaces behind stable behavior contracts.
**Depends on**: Phase 38
**Requirements**: [STRC-01, STRC-02]
**Success Criteria** (what must be TRUE):
  1. Priority oversized modules are broken into smaller bounded units with clearer ownership.
  2. Cleanup-sensitive Rust, sidecar, and operator-surface contracts are easier to trace after the refactor.
  3. Shipped runtime and operator behavior stays intact while internals become easier to navigate.
**Plans**: Not started

### Phase 40: Cleanup Verification and Maintenance Guardrails
**Goal**: Make the cleanup durable by proving the refactor did not regress core behavior and by keeping the remaining debt visible.
**Depends on**: Phase 39
**Requirements**: [SAFE-01, SAFE-02]
**Success Criteria** (what must be TRUE):
  1. Cleanup-sensitive surfaces have a targeted verification bundle that can be rerun later.
  2. Remaining cleanup debt is recorded in one maintained artifact instead of being rediscovered piecemeal.
  3. The next milestone inherits a cleaner and safer baseline rather than reopening the same structural drift.
**Plans**: Not started

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 37. Codebase Cleanup Inventory and Refactor Contract | 2/2 | Complete | 2026-03-27 |
| 38. Repo Hygiene and Drift Reduction | 2/2 | Complete | 2026-03-27 |
| 39. Command Surface Decomposition and Boundary Cleanup | 0/0 | Not Started | - |
| 40. Cleanup Verification and Maintenance Guardrails | 0/0 | Not Started | - |

## Phase History

<details>
<summary>🚧 v1.8 Clean Codebase — ACTIVE</summary>

- [x] **Phase 37: Codebase Cleanup Inventory and Refactor Contract** — completed 2026-03-27
- [x] **Phase 38: Repo Hygiene and Drift Reduction** — completed 2026-03-27
- [ ] **Phase 39: Command Surface Decomposition and Boundary Cleanup**
- [ ] **Phase 40: Cleanup Verification and Maintenance Guardrails**

</details>

<details>
<summary>✅ v1.7 Documentation Convergence and OpenClaw-Inspired Docs Rewrite — SHIPPED 2026-03-27</summary>

- [x] **Phase 32: Documentation Inventory and Canonical Source Contract** — completed 2026-03-27
- [x] **Phase 33: README and Documentation Entry Surface Rewrite** — completed 2026-03-27
- [x] **Phase 34: Getting Started and Setup Guide Convergence** — completed 2026-03-27
- [x] **Phase 35: Operator, Deployment, and Planning Docs Sync** — completed 2026-03-27
- [x] **Phase 36: Documentation Governance and Drift Prevention** — completed 2026-03-27

</details>

<details>
<summary>✅ v1.6 Proper Onboarding and Setup — SHIPPED 2026-03-28</summary>

- [x] **Phase 28: Setup State and Resumable Onboarding Contract** — completed 2026-03-28
- [x] **Phase 29: Mode-Aware Provider, Runtime, and Channel Bootstrap** — completed 2026-03-28
- [x] **Phase 30: Setup Repair and Existing Workspace Recovery** — completed 2026-03-28
- [x] **Phase 31: Setup Handoff and Operator Surface Alignment** — completed 2026-03-28

</details>

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
