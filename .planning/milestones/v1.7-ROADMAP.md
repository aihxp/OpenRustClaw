# Roadmap: OpenRustClaw

## Milestones

- ✅ **v1.0 Rust OpenClaw MVP** — shipped 2026-03-26. Archive: `.planning/milestones/v1.0-ROADMAP.md`
- ✅ **v1.1 Lifecycle Integrity and Enterprise Foundations** — shipped 2026-03-26. Archive: `.planning/milestones/v1.1-ROADMAP.md`
- ✅ **v1.2 Deeper OpenClaw Surface Parity** — shipped 2026-03-27. Archive: `.planning/milestones/v1.2-ROADMAP.md`
- ✅ **v1.3 Enterprise Expansion and Supervised Autonomy Foundations** — shipped 2026-03-27. Archive: `.planning/milestones/v1.3-ROADMAP.md`
- ✅ **v1.4 Enterprise Governance and Operator-Gated Full Autonomy** — shipped 2026-03-27. Archive: `.planning/milestones/v1.4-ROADMAP.md`
- ✅ **v1.5 Self-Hosted Product Modes and Lifecycle Packaging** — shipped 2026-03-27. Archive: `.planning/milestones/v1.5-ROADMAP.md`
- ✅ **v1.6 Proper Onboarding and Setup** — shipped 2026-03-28. Archive: `.planning/milestones/v1.6-ROADMAP.md`
- 🚧 **v1.7 Documentation Convergence and OpenClaw-Inspired Docs Rewrite** — phases 32-36

## Roadmap v1.7: Documentation Convergence and OpenClaw-Inspired Docs Rewrite

### Overview

This milestone treats documentation as a real product surface. The order is deliberate: first establish a canonical docs contract and inventory so the repo stops pretending multiple overlapping files are equally authoritative, then rewrite the README and documentation entry surfaces around one clearer self-hosted product story, then converge setup and getting-started guidance, then rewrite operator and planning docs against the shipped runtime surface, and finally lock the result in with a docs-governance and drift-prevention contract.

### Phases

**Phase Numbering:**
- Integer phases continue across milestones to preserve one linear execution history.
- Decimal phases are reserved for urgent insertions if roadmap assumptions break.

- [x] **Phase 32: Documentation Inventory and Canonical Source Contract** - completed 2026-03-27. Documentation ownership, mirror rules, and cleanup criteria are now explicit.
- [x] **Phase 33: README and Documentation Entry Surface Rewrite** - completed 2026-03-27. The repo and mdBook entry surfaces now tell one clearer self-hosted product story.
- [x] **Phase 34: Getting Started and Setup Guide Convergence** - completed 2026-03-27. The getting-started path now shares one setup lifecycle and handoff story.
- [x] **Phase 35: Operator, Deployment, and Planning Docs Sync** - completed 2026-03-27. Operator guides and canonical planning docs now match the rewritten product story.
- [x] **Phase 36: Documentation Governance and Drift Prevention** - completed 2026-03-27. The docs contract, audit, and cleanup rules now make the rewrite durable.

### Phase Details

### Phase 32: Documentation Inventory and Canonical Source Contract
**Goal**: Define the documentation ownership model so future rewrites converge existing files instead of adding another parallel layer.
**Depends on**: v1.6 archive state
**Requirements**: [DOCS-01, DOCS-02]
**Success Criteria** (what must be TRUE):
  1. The repo identifies which files are canonical, mirrored, derived, or candidates for deletion.
  2. Duplicate or stale docs families are inventory-backed rather than handled ad hoc.
  3. The milestone has an explicit merge/delete target list before the rewrite begins.
**Plans**: 2 plans complete

Plans:
- [x] 32-01 Audit README, repo-root docs, and docs-site overlap
- [x] 32-02 Define canonical ownership and cleanup rules

### Phase 33: README and Documentation Entry Surface Rewrite
**Goal**: Make the first impression legible so OpenRustClaw reads like one coherent self-hosted product instead of a broad codebase dump.
**Depends on**: Phase 32
**Requirements**: [ENTRY-01, ENTRY-02]
**Success Criteria** (what must be TRUE):
  1. README and docs landing surfaces tell one consistent product story.
  2. New users can identify what OpenRustClaw is, who it is for, and how to start.
  3. The rewrite borrows the clarity of OpenClaw’s public docs style without drifting into inaccurate or copied claims.
**Plans**: 2 plans complete

Plans:
- [x] 33-01 Rewrite README around the self-hosted product story
- [x] 33-02 Rewrite docs introduction and primary navigation entry points

### Phase 34: Getting Started and Setup Guide Convergence
**Goal**: Ensure setup-facing docs all describe the same shipped lifecycle instead of sending operators through conflicting paths.
**Depends on**: Phase 33
**Requirements**: [SETUP-01]
**Success Criteria** (what must be TRUE):
  1. Installation, quickstart, first-agent, and setup handoff pages all agree on the setup path.
  2. Standard, Advanced, and Custom setup depth plus repair and upgrade or downgrade expectations are documented consistently.
  3. The getting-started path reads like one guided flow instead of several disconnected docs.
**Plans**: 2 plans complete

Plans:
- [x] 34-01 Converge install and quickstart guidance
- [x] 34-02 Align first-agent and setup handoff docs with shipped setup surfaces

### Phase 35: Operator, Deployment, and Planning Docs Sync
**Goal**: Rewrite operator-facing and planning-facing documentation so the shipped runtime and control surfaces are described truthfully and compactly.
**Depends on**: Phase 34
**Requirements**: [OPS-01, SURF-01]
**Success Criteria** (what must be TRUE):
  1. Deployment, production, security, observability, and release guidance reflect the shipped product.
  2. Feature matrix, surface matrix, roadmap, and product-positioning pages fit the same product story and navigation model.
  3. Stale promises or outdated matrix rows are removed rather than preserved for historical comfort.
**Plans**: 2 plans complete

Plans:
- [x] 35-01 Rewrite operator docs around the current runtime and control surface
- [x] 35-02 Sync planning and matrix docs with shipped behavior

### Phase 36: Documentation Governance and Drift Prevention
**Goal**: Make the docs rewrite durable by defining how future milestones merge, delete, verify, and keep canonical docs synchronized.
**Depends on**: Phase 35
**Requirements**: [GOV-01]
**Success Criteria** (what must be TRUE):
  1. The docs set has explicit maintenance and ownership rules.
  2. Cleanup decisions from earlier phases are fully applied or documented.
  3. Future milestones have a clear rule for how to update docs without recreating parallel drift.
**Plans**: 2 plans complete

Plans:
- [x] 36-01 Add documentation maintenance contract
- [x] 36-02 Complete cleanup, redirects, and verification sync

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 32. Documentation Inventory and Canonical Source Contract | 2/2 | Complete | 2026-03-27 |
| 33. README and Documentation Entry Surface Rewrite | 2/2 | Complete | 2026-03-27 |
| 34. Getting Started and Setup Guide Convergence | 2/2 | Complete | 2026-03-27 |
| 35. Operator, Deployment, and Planning Docs Sync | 2/2 | Complete | 2026-03-27 |
| 36. Documentation Governance and Drift Prevention | 2/2 | Complete | 2026-03-27 |

## Current Status

- Active milestone: v1.7 Documentation Convergence and OpenClaw-Inspired Docs Rewrite
- Next step: `$gsd-audit-milestone`
- Archived milestone planning artifacts live under `.planning/milestones/`.

## Phase History

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
