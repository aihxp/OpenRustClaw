# Roadmap: OpenRustClaw

## Milestones

- ✅ **v1.0 Rust OpenClaw MVP** — shipped 2026-03-26. Archive: `.planning/milestones/v1.0-ROADMAP.md`
- ✅ **v1.1 Lifecycle Integrity and Enterprise Foundations** — shipped 2026-03-26. Archive: `.planning/milestones/v1.1-ROADMAP.md`
- 🚧 **v1.2 Deeper OpenClaw Surface Parity** — phases 11-15

## Roadmap v1.2: Deeper OpenClaw Surface Parity

### Overview

This milestone deepens the five highest-value OpenClaw parity surfaces without abandoning the trust-first baseline already shipped. The order is deliberate: first deepen browser automation and supervision where parity unlocks broad operator value, then expand mobile and Control UI depth around those contracts, and finally close the slice with richer voice and call-handling parity.

### Phases

**Phase Numbering:**
- Integer phases continue across milestones to preserve one linear execution history.
- Decimal phases are reserved for urgent insertions if roadmap assumptions break.

- [x] **Phase 11: Browser Automation Depth** - Deepen browser automation parity with richer multi-step workflows, artifacts, and inspection without weakening the browser trust contract.
- [ ] **Phase 12: Multi-Agent Supervision Parity** - Expand orchestration and delegated-run supervision so multi-agent behavior is easier to inspect, control, and trust.
- [ ] **Phase 13: Mobile Runtime Parity** - Broaden mobile runtime and operator parity while preserving approval gates and durable receipts.
- [ ] **Phase 14: Control UI Surface Completion** - Bring the shipped Control UI closer to OpenClaw parity across the deeper browser, supervision, and mobile surfaces.
- [ ] **Phase 15: Voice and Call Handling Parity** - Deepen voice and call-handling parity with richer operator-visible control, health, and evidence.

### Phase Details

### Phase 11: Browser Automation Depth
**Goal**: Deepen browser workflow parity with richer multi-step automation, artifacts, and inspection over the shipped browser lane.
**Depends on**: v1.1 archive state
**Requirements**: [BROW-01, BROW-02]
**Success Criteria** (what must be TRUE):
  1. Operators can run deeper browser workflows than the current basic browse-and-inspect lane.
  2. The richer browser lane still produces durable artifacts and operator inspection evidence.
  3. Browser depth continues to honor the explicit browser policy and audit boundary.
**Plans**: 3 complete

Plans:
- [x] **11-01** Add a durable browser workflow history ledger
- [x] **11-02** Surface browser workflow history in runtime APIs and Control UI
- [x] **11-03** Align browser parity docs and preserve verification

### Phase 12: Multi-Agent Supervision Parity
**Goal**: Expand multi-agent and orchestration supervision parity so delegated work is easier to inspect and control.
**Depends on**: Phase 11
**Requirements**: [SUPR-01, SUPR-02]
**Success Criteria** (what must be TRUE):
  1. Operators can inspect more of the delegated-run lifecycle than final receipts alone.
  2. Supervision parity preserves approval, trace, and resource visibility as delegation deepens.
  3. The expanded supervision lane remains operator-readable from shipped surfaces.
**Plans**: TBD

Plans:
- [ ] TBD (run `$gsd-plan-phase 12` to break down)

### Phase 13: Mobile Runtime Parity
**Goal**: Broaden mobile runtime and operator parity so the shipped mobile lane feels materially closer to OpenClaw.
**Depends on**: Phase 12
**Requirements**: [MOBL-01, MOBL-02]
**Success Criteria** (what must be TRUE):
  1. Operators can inspect and act on a broader mobile runtime surface than the current bounded lane.
  2. Mobile expansions keep approval gates, receipts, and evidence first-class.
  3. Mobile parity work integrates with the existing trust-first control plane rather than bypassing it.
**Plans**: TBD

Plans:
- [ ] TBD (run `$gsd-plan-phase 13` to break down)

### Phase 14: Control UI Surface Completion
**Goal**: Bring the shipped Control UI closer to OpenClaw parity across the deeper runtime surfaces.
**Depends on**: Phase 13
**Requirements**: [CTRL-01, CTRL-02]
**Success Criteria** (what must be TRUE):
  1. Control UI covers more of the parity-critical surfaces operators actually use.
  2. The deeper UI remains backed by typed runtime contracts instead of frontend-only reconstruction.
  3. The operator dashboard becomes a more complete parity surface rather than a partial monitor.
**Plans**: TBD

Plans:
- [ ] TBD (run `$gsd-plan-phase 14` to break down)

### Phase 15: Voice and Call Handling Parity
**Goal**: Deepen voice and call-handling parity with richer inspection, control, and durable evidence.
**Depends on**: Phase 14
**Requirements**: [VOIC-01, VOIC-02]
**Success Criteria** (what must be TRUE):
  1. Operators can inspect and manage richer voice or call-handling flows than the current MVP lane.
  2. Voice and call parity surfaces preserve session health, artifacts, and operator evidence.
  3. The resulting voice surface feels closer to real OpenClaw parity while staying bounded and truthful.
**Plans**: TBD

Plans:
- [ ] TBD (run `$gsd-plan-phase 15` to break down)

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 11. Browser Automation Depth | 3/3 | Complete    | 2026-03-26 |
| 12. Multi-Agent Supervision Parity | 0/TBD | Not started | - |
| 13. Mobile Runtime Parity | 0/TBD | Not started | - |
| 14. Control UI Surface Completion | 0/TBD | Not started | - |
| 15. Voice and Call Handling Parity | 0/TBD | Not started | - |

## Phase History

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

## Current Status

- Active milestone: v1.2 Deeper OpenClaw Surface Parity
- Next step: `$gsd-discuss-phase 12` or `$gsd-plan-phase 12`
- Archived v1.0 and v1.1 planning artifacts live under `.planning/milestones/`.
