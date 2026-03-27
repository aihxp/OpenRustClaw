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
- ◆ **v1.9 GitHub Repository Presence and Actions Recovery** — in progress

## Current Status

- Active milestone: **v1.9 GitHub Repository Presence and Actions Recovery**
- Progress: 0 of 4 phases complete (0%)
- Next phase: **Phase 41**
- Next step: `$gsd-plan-phase 41` or `$gsd-autonomous`

## Roadmap v1.9: GitHub Repository Presence and Actions Recovery

**Goal:** Make the public GitHub repo surface truthful and current, add a maintained discovery-tag contract, and repair GitHub Actions so the repo's automation story matches the shipped product.

### Overview

This milestone is about making the GitHub repo itself truthful and operational again. The order is deliberate: first align the public repo framing with the shipped product so the entry surface stops misleading operators, then define the discovery tag contract, then repair the automation surface so badges and workflow status mean something again, and finally close with a repeatable admin sync and verification bundle for future GitHub maintenance.

### Phases

**Phase Numbering:**
- Integer phases continue across milestones to preserve one linear execution history.
- Decimal phases are reserved for urgent insertions if roadmap assumptions break.

- [ ] **Phase 41: GitHub About and Public Positioning Contract** - Align the repo About language, linked entry surface, README badge targets, and public-facing guidance with the current self-hosted Rust-first product story.
- [ ] **Phase 42: GitHub Topics and Discovery Surface** - Define the canonical GitHub topic or word-tag set, document it in-repo, and create a safe sync path for keeping repo discovery metadata aligned.
- [ ] **Phase 43: GitHub Actions Audit and Repair** - Audit the live Actions surface against local workflow files, remove or fix stale jobs, and make badges, workflow names, and release automation match the current verification contract.
- [ ] **Phase 44: GitHub Admin Sync and Verification Exit** - Add a repeatable admin checklist or sync path for repo metadata and workflow health, then close the milestone with local plus live GitHub verification evidence.

### Phase Details

### Phase 41: GitHub About and Public Positioning Contract
**Goal**: Replace stale GitHub repo framing with a public entry surface that matches the shipped self-hosted Rust-first product.
**Depends on**: v1.8 archive state
**Requirements**: [GHMD-01, GHMD-02]
**Success Criteria** (what must be TRUE):
  1. The public repo About wording, README entry surface, and linked resources describe the same current product.
  2. The repo no longer advertises the stale hybrid-framework positioning that conflicts with shipped docs.
  3. Repo entry links and badges point at the current documentation and automation surfaces.
**Plans**: 2 plans pending

Plans:
- [ ] 41-01 Align GitHub About wording with the current product story
- [ ] 41-02 Normalize README badges and public entry links

### Phase 42: GitHub Topics and Discovery Surface
**Goal**: Make repo discovery metadata intentional and repeatable instead of ad hoc or stale.
**Depends on**: Phase 41
**Requirements**: [DISC-01, DISC-02]
**Success Criteria** (what must be TRUE):
  1. OpenRustClaw has a canonical topic or word-tag set that reflects the shipped product.
  2. The topic or tag set is documented in-repo and safe to reapply later.
  3. Discovery metadata does not drift independently from the product story.
**Plans**: 2 plans pending

Plans:
- [ ] 42-01 Define the canonical GitHub topic or tag set
- [ ] 42-02 Add repo-admin sync guidance for discovery metadata

### Phase 43: GitHub Actions Audit and Repair
**Goal**: Restore confidence in the public automation surface by making workflows, badges, and release jobs reflect the current verification contract.
**Depends on**: Phase 42
**Requirements**: [ACT-01, ACT-02, ACT-03]
**Success Criteria** (what must be TRUE):
  1. Public workflow names, badges, and run expectations map cleanly to the current repo layout and verification bundle.
  2. Stale or failing workflow paths are fixed, removed, or clearly downgraded.
  3. Release or tag automation remains consistent with shipped milestone tags and release artifacts.
**Plans**: 2 plans pending

Plans:
- [ ] 43-01 Audit live GitHub Actions against local workflow definitions
- [ ] 43-02 Repair workflow, badge, and release-contract drift

### Phase 44: GitHub Admin Sync and Verification Exit
**Goal**: Close the milestone with one repeatable path for future GitHub metadata and Actions maintenance.
**Depends on**: Phase 43
**Requirements**: [OPS-01, OPS-02]
**Success Criteria** (what must be TRUE):
  1. Repo-admin metadata and workflow sync has one documented repeatable process.
  2. The milestone records both local validation and live GitHub surface evidence.
  3. Future repo-maintenance work can build on a truthful GitHub baseline rather than rediscovering drift.
**Plans**: 2 plans pending

Plans:
- [ ] 44-01 Add GitHub admin sync and verification checklist
- [ ] 44-02 Close the milestone with live GitHub surface evidence

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 41. GitHub About and Public Positioning Contract | 0/2 | Pending | - |
| 42. GitHub Topics and Discovery Surface | 0/2 | Pending | - |
| 43. GitHub Actions Audit and Repair | 0/2 | Pending | - |
| 44. GitHub Admin Sync and Verification Exit | 0/2 | Pending | - |

## Phase History

<details>
<summary>🚧 v1.9 GitHub Repository Presence and Actions Recovery — ACTIVE</summary>

- [ ] **Phase 41: GitHub About and Public Positioning Contract**
- [ ] **Phase 42: GitHub Topics and Discovery Surface**
- [ ] **Phase 43: GitHub Actions Audit and Repair**
- [ ] **Phase 44: GitHub Admin Sync and Verification Exit**

</details>

<details>
<summary>✅ v1.8 Clean Codebase — SHIPPED 2026-03-27</summary>

- [x] **Phase 37: Codebase Cleanup Inventory and Refactor Contract** — completed 2026-03-27
- [x] **Phase 38: Repo Hygiene and Drift Reduction** — completed 2026-03-27
- [x] **Phase 39: Command Surface Decomposition and Boundary Cleanup** — completed 2026-03-27
- [x] **Phase 40: Cleanup Verification and Maintenance Guardrails** — completed 2026-03-27

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
