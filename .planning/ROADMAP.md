# Roadmap: OpenRustClaw

## Milestones

- ✅ **v1.0 through v1.42** - shipped. Full milestone history and archives: `.planning/MILESTONES.md`
- 🚧 **v1.43 Learning Loop, Memory Depth, and God Mode** - Phases 181-185

## Overview

`v1.43` stays bounded to trustworthy memory depth, reviewable learning, reusable skill improvement, and a distinct God Mode overlay. It improves retrieval and consolidation first, keeps learning candidate-before-promotion, routes skill improvement through the existing approval and compile path, and leaves God Mode as the final phase so higher-power execution does not arrive before stronger audit and rollback controls.

## Phases

**Phase Numbering:**
- Integer phases continue the live sequence from the prior milestone.
- This milestone starts at Phase 181 because `v1.42` ended at Phase 180.

- [x] **Phase 181: Hybrid Retrieval and Recall Inspection** - Make recall retrieval, assembly, and inspection reliable and explainable across sessions.
- [x] **Phase 182: Structured Memory Artifacts and Model Control** - Promote durable user, operator, project, and archive artifacts through bounded policy-gated memory flows.
- [x] **Phase 183: Learning Candidate Review and Lesson Promotion** - Turn successful work and reflections into reviewable candidates that can become bounded active lessons.
- [x] **Phase 184: Skill Proposal Verification and Reuse** - Generate reviewable reusable-skill proposals and route approved ones through the existing compile and install path.
- [ ] **Phase 185: God Mode Overlay, Audit, and Recovery** - Add a distinct full-power operator lane that remains explicit, auditable, and reversible.

## Phase Details

### Phase 181: Hybrid Retrieval and Recall Inspection
**Goal**: Recall surfaces the most relevant prior context with explainable ranking and bounded assembled output instead of opaque or brittle memory retrieval.
**Depends on**: Nothing (first phase)
**Requirements**: RETR-01, RETR-02, RETR-03, RETR-04
**Success Criteria** (what must be TRUE):
  1. Recall results are ranked using lexical, vector, recency, confidence, and importance signals instead of a flattened or opaque score.
  2. Retrieved memory is assembled into concise, deduplicated output that includes provenance, freshness, and artifact-type metadata for each surfaced item.
  3. Operators can inspect why a memory was surfaced, including the ranking factors and contributing source artifacts.
  4. The runtime keeps the recall-only memory contract by using bounded recall summaries and never injecting raw memory files or raw archive blobs into the system prompt.
**Plans**: TBD

### Phase 182: Structured Memory Artifacts and Model Control
**Goal**: The runtime maintains distinct durable memory artifacts and projects only a bounded high-signal subset into active context.
**Depends on**: Phase 181
**Requirements**: MODL-01, MODL-02, MODL-03, MODL-04
**Success Criteria** (what must be TRUE):
  1. User model, operator model, project memory, and archive summaries exist as distinct durable artifact classes instead of one blended profile.
  2. Memory consolidation creates summaries and model artifacts only through explicit policy-gated promotion with source lineage.
  3. The runtime projects only a bounded high-signal subset of structured model artifacts into core memory or prompt context.
  4. Operators can inspect, correct, deactivate, or remove stale, wrong, or unsafe model artifacts without editing raw memory files directly.
**Plans**: TBD

### Phase 183: Learning Candidate Review and Lesson Promotion
**Goal**: Successful work, reflections, and audit evidence become reviewable learning candidates that can promote into bounded runtime guidance only with explicit evidence and rollback discipline.
**Depends on**: Phase 182
**Requirements**: LEAR-01, LEAR-02, LEAR-03, LEAR-04
**Success Criteria** (what must be TRUE):
  1. Successful runs, reflections, and relevant audit evidence can create durable learning candidates with provenance, confidence, and review state.
  2. Learning candidates can be approved, rejected, superseded, or rolled back before they become active lessons or memory artifacts.
  3. Promoted lessons can guide routing, recall, tool choice, or other bounded runtime decisions without silently widening authority.
  4. High-impact learned artifacts require replay, evaluation, or equivalent review evidence before promotion.
**Plans**: TBD

### Phase 184: Skill Proposal Verification and Reuse
**Goal**: Repeated successful workflows generate reviewable skill proposals that remain inactive until they are verified, approved, and installed through the existing skill pipeline.
**Depends on**: Phase 183
**Requirements**: SKIL-01, SKIL-02, SKIL-03
**Success Criteria** (what must be TRUE):
  1. Repeated successful workflows can produce reviewable proposals for new reusable skills or improvements to existing skills.
  2. Skill proposals remain human-readable, diffable, and inactive until they pass verification and explicit approval.
  3. Approved skill proposals flow through the existing compile and install path with durable provenance back to the source candidate or lesson.
**Plans**: 184-01, 184-02

### Phase 185: God Mode Overlay, Audit, and Recovery
**Goal**: Operators can use a distinct God Mode lane with full autonomy, full access, and full power without weakening the default trust-first runtime path.
**Depends on**: Phase 184
**Requirements**: GOD-01, GOD-02, GOD-03, GOD-04
**Success Criteria** (what must be TRUE):
  1. Operators can explicitly enable a distinct `God Mode` lane instead of inheriting it from normal runtime behavior.
  2. God Mode activation uses explicit scope, TTL or session boundaries, baseline-restore behavior, and a kill-switch control.
  3. God Mode runs and any learned artifacts they produce are prominently labeled, auditable, and quarantine-capable.
  4. Disabling or expiring God Mode returns the workspace to the default trust-first runtime without implicit retention of God Mode permissions, approval bypasses, or tool grants.
**Plans**: TBD

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 181. Hybrid Retrieval and Recall Inspection | 2/2 | Complete | 2026-04-08 |
| 182. Structured Memory Artifacts and Model Control | 2/2 | Complete | 2026-04-08 |
| 183. Learning Candidate Review and Lesson Promotion | 2/2 | Complete | 2026-04-08 |
| 184. Skill Proposal Verification and Reuse | 2/2 | Complete | 2026-04-08 |
| 185. God Mode Overlay, Audit, and Recovery | 0/0 | Not started | - |

## Current Status

- Active milestone: v1.43 Learning Loop, Memory Depth, and God Mode
- Roadmap progress: 4/5 phases complete
- Current work: Phase 184 is complete across durable proposal storage, inactive artifact verification, CLI and MCP proposal flows, and the active skill install and rollback bridge
- Next step: `$gsd-discuss-phase 185 --auto`
