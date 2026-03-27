# Roadmap: OpenRustClaw

## Milestones

- ✅ **v1.0 Rust OpenClaw MVP** — shipped 2026-03-26. Archive: `.planning/milestones/v1.0-ROADMAP.md`
- ✅ **v1.1 Lifecycle Integrity and Enterprise Foundations** — shipped 2026-03-26. Archive: `.planning/milestones/v1.1-ROADMAP.md`
- ✅ **v1.2 Deeper OpenClaw Surface Parity** — shipped 2026-03-27. Archive: `.planning/milestones/v1.2-ROADMAP.md`
- ✅ **v1.3 Enterprise Expansion and Supervised Autonomy Foundations** — shipped 2026-03-27. Archive: `.planning/milestones/v1.3-ROADMAP.md`
- 🚧 **v1.4 Enterprise Governance and Operator-Gated Full Autonomy** — phases 20-23

## Roadmap v1.4: Enterprise Governance and Operator-Gated Full Autonomy

### Overview

This milestone extends the enterprise baseline from v1.3 into stronger governance and audit posture, then adds the user-requested “god mode” as a separate operator-gated full-autonomy lane. The order is deliberate: first deepen governance boundaries, then strengthen audit and retention, then add the explicit autonomy override, and finally close with one shipped control surface that keeps the stronger autonomy lane inspectable and kill-switchable.

### Phases

**Phase Numbering:**
- Integer phases continue across milestones to preserve one linear execution history.
- Decimal phases are reserved for urgent insertions if roadmap assumptions break.

- [x] **Phase 20: Enterprise Governance and Approval Chains** - completed 2026-03-27. Deepened role, approval, and separation-of-duties controls for sensitive enterprise actions.
- [x] **Phase 21: Enterprise Audit Retention and Review Packaging** - completed 2026-03-27. Strengthened retention, export, and enterprise reviewability for governance and autonomy evidence.
- [x] **Phase 22: Operator-Gated Full Autonomy Mode** - completed 2026-03-27. Added a dedicated enterprise full-autonomy lane with explicit enable or disable or kill-switch state, dedicated governance scope, and durable audit evidence.
- [ ] **Phase 23: Enterprise Autonomy Control Surface** - Make governance and full-autonomy controls usable and inspectable from shipped operator surfaces.

### Phase Details

### Phase 20: Enterprise Governance and Approval Chains
**Goal**: Deepen enterprise governance beyond the current baseline with stronger role boundaries, approval chains, and separation-of-duties controls.
**Depends on**: v1.3 archive state
**Requirements**: [GOV-01, GOV-02]
**Success Criteria** (what must be TRUE):
  1. Sensitive enterprise actions can express stronger approval and role boundaries than the current flat operator model.
  2. Governance controls preserve separation of duties for higher-risk actions.
  3. The deeper governance contract remains inspectable from the Rust-owned control plane.
**Plans**: 3 plans complete

Plans:
- [x] 20-01 Add A Typed Enterprise Governance Contract
- [x] 20-02 Enforce Approval Chains For Sensitive Enterprise Writes
- [x] 20-03 Make Governance Usable From The Shipped Control Surface

### Phase 21: Enterprise Audit Retention and Review Packaging
**Goal**: Expand enterprise audit and retention handling so governance and autonomy evidence is more useful for enterprise review and operations.
**Depends on**: Phase 20
**Requirements**: [AUD-01, AUD-02]
**Success Criteria** (what must be TRUE):
  1. Enterprise evidence exports preserve richer governance and operator context.
  2. Audit retention and export behavior is configurable from one coherent enterprise surface.
  3. Operators can review the strengthened audit contract without scraping multiple raw ledgers.
**Plans**: 3 plans complete

Plans:
- [x] 21-01 Deepen Enterprise Audit Retention Policy
- [x] 21-02 Package Governance And Autonomy Evidence For Review
- [x] 21-03 Surface Enterprise Audit Review In Control UI

### Phase 22: Operator-Gated Full Autonomy Mode
**Goal**: Add an explicit full-autonomy override lane that trusted operators can enable deliberately without changing the default trust-first runtime path.
**Depends on**: Phase 21
**Requirements**: [AUTO-05, AUTO-06]
**Success Criteria** (what must be TRUE):
  1. Full autonomy is an explicit operator-gated mode rather than a hidden default behavior change.
  2. Full-autonomy runs preserve budgets, kill switches, and durable enablement or shutdown evidence.
  3. The stronger autonomy lane remains operator-governed and reversible rather than opaque.
**Plans**: 3 plans complete

Plans:
- [x] 22-01 Add A Durable Full-Autonomy Override Contract
- [x] 22-02 Enforce Full-Autonomy Enablement, Disable, And Kill-Switch Actions
- [x] 22-03 Close The Backend Full-Autonomy Lane With Audit And Docs

### Phase 23: Enterprise Autonomy Control Surface
**Goal**: Make the enterprise governance and full-autonomy override lane usable and governable from shipped control surfaces.
**Depends on**: Phase 22
**Requirements**: [ADMN-02]
**Success Criteria** (what must be TRUE):
  1. Operators can inspect, configure, and shut down the full-autonomy lane from shipped control surfaces.
  2. The admin surface stays grounded in typed runtime contracts rather than frontend-only stitching.
  3. Docs and verification close the milestone with a truthful enterprise-autonomy baseline.
**Plans**: 3 plans complete

Plans:
- [ ] 23-01 Surface Full-Autonomy Inspection In Control UI
- [ ] 23-02 Add Enable, Disable, And Kill-Switch Controls To The Shipped Admin Surface
- [ ] 23-03 Close The Enterprise Autonomy Operator Loop

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 20. Enterprise Governance and Approval Chains | 3/3 | Complete | 2026-03-27 |
| 21. Enterprise Audit Retention and Review Packaging | 3/3 | Complete | 2026-03-27 |
| 22. Operator-Gated Full Autonomy Mode | 3/3 | Complete | 2026-03-27 |
| 23. Enterprise Autonomy Control Surface | 0/3 | Planned | - |

## Current Status

- Active milestone: v1.4 Enterprise Governance and Operator-Gated Full Autonomy
- Next step: `$gsd-execute-phase 23`
- Archived milestone planning artifacts live under `.planning/milestones/`.

## Phase History

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
