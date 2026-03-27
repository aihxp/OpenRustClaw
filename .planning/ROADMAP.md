# Roadmap: OpenRustClaw

## Milestones

- ✅ **v1.0 Rust OpenClaw MVP** — shipped 2026-03-26. Archive: `.planning/milestones/v1.0-ROADMAP.md`
- ✅ **v1.1 Lifecycle Integrity and Enterprise Foundations** — shipped 2026-03-26. Archive: `.planning/milestones/v1.1-ROADMAP.md`
- ✅ **v1.2 Deeper OpenClaw Surface Parity** — shipped 2026-03-27. Archive: `.planning/milestones/v1.2-ROADMAP.md`
- ✅ **v1.3 Enterprise Expansion and Supervised Autonomy Foundations** — shipped 2026-03-27. Archive: `.planning/milestones/v1.3-ROADMAP.md`
- ✅ **v1.4 Enterprise Governance and Operator-Gated Full Autonomy** — shipped 2026-03-27. Archive: `.planning/milestones/v1.4-ROADMAP.md`

## Roadmap v1.5: Self-Hosted Product Modes and Lifecycle Packaging

### Overview

This milestone makes OpenRustClaw legible as one self-hosted open-source product that can fit a solo user, a multi-user team, a company, or an enterprise deployment. The order is deliberate: first define the deployment-mode contract, then branch onboarding around those modes, then make upgrade and downgrade transitions explicit, and finally align the docs and shipped control surfaces around the same product story.

### Phases

**Phase Numbering:**
- Integer phases continue across milestones to preserve one linear execution history.
- Decimal phases are reserved for urgent insertions if roadmap assumptions break.

- [ ] **Phase 24: Self-Hosted Product Modes and Instance Profile Contract** - Define explicit deployment modes and the instance-profile contract behind them.
- [ ] **Phase 25: Tiered Onboarding and First-Run Paths** - Add differentiated onboarding flows for solo, multi-user team, company, and enterprise installs.
- [ ] **Phase 26: Upgrade and Downgrade Lifecycle** - Add explicit upgrade and downgrade paths between deployment modes.
- [ ] **Phase 27: Self-Hosted Product Surface Alignment** - Align docs and shipped operator surfaces around the current mode and transition path.

### Phase Details

### Phase 24: Self-Hosted Product Modes and Instance Profile Contract
**Goal**: Define the product-mode and instance-profile contract so OpenRustClaw can speak clearly about solo, multi-user team, company, and enterprise deployments.
**Depends on**: v1.4 archive state
**Requirements**: [MODE-01, MODE-02]
**Success Criteria** (what must be TRUE):
  1. The product is explicitly framed as self-hosted and open-source across the planning contract.
  2. Deployment mode is represented as a first-class profile rather than an implied collection of toggles.
  3. The mode contract is structured so onboarding and lifecycle transitions can build on it cleanly.
**Plans**: TBD

Plans:
- [ ] TBD (run `$gsd-plan-phase 24` to break down)

### Phase 25: Tiered Onboarding and First-Run Paths
**Goal**: Add differentiated first-run onboarding paths for the core deployment modes.
**Depends on**: Phase 24
**Requirements**: [ONBR-01, ONBR-02]
**Success Criteria** (what must be TRUE):
  1. First-run setup offers different paths for solo, multi-user team, company, and enterprise installs.
  2. Each path explains its trust boundary, setup steps, and recommended defaults honestly.
  3. The onboarding flow remains grounded in the Rust-owned setup surfaces rather than marketing-only copy.
**Plans**: TBD

Plans:
- [ ] TBD (run `$gsd-plan-phase 25` to break down)

### Phase 26: Upgrade and Downgrade Lifecycle
**Goal**: Add safe lifecycle transitions between deployment modes.
**Depends on**: Phase 25
**Requirements**: [LIFE-01, LIFE-02]
**Success Criteria** (what must be TRUE):
  1. Operators can upgrade or downgrade between modes through explicit, reviewable transitions.
  2. Mode changes surface governance, permission, and retained-data implications rather than hiding them.
  3. Transition handling preserves the existing trust and audit contract instead of bypassing it.
**Plans**: TBD

Plans:
- [ ] TBD (run `$gsd-plan-phase 26` to break down)

### Phase 27: Self-Hosted Product Surface Alignment
**Goal**: Align docs and shipped operator surfaces around the selected deployment mode and its transition story.
**Depends on**: Phase 26
**Requirements**: [SURF-01]
**Success Criteria** (what must be TRUE):
  1. Docs and shipped control surfaces consistently present OpenRustClaw as a self-hosted open-source product.
  2. The current deployment mode and transition path are visible from operator-facing surfaces.
  3. The milestone closes with a truthful product-mode baseline rather than disconnected docs and setup behavior.
**Plans**: TBD

Plans:
- [ ] TBD (run `$gsd-plan-phase 27` to break down)

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 24. Self-Hosted Product Modes and Instance Profile Contract | 0/TBD | Not started | - |
| 25. Tiered Onboarding and First-Run Paths | 0/TBD | Not started | - |
| 26. Upgrade and Downgrade Lifecycle | 0/TBD | Not started | - |
| 27. Self-Hosted Product Surface Alignment | 0/TBD | Not started | - |

## Current Status

- Active milestone: v1.5 Self-Hosted Product Modes and Lifecycle Packaging
- Next step: `$gsd-discuss-phase 24` or `$gsd-plan-phase 24`
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
