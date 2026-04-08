# Requirements: OpenRustClaw v1.43 Learning Loop, Memory Depth, and God Mode

**Defined:** 2026-04-07
**Core Value:** Ship a trustworthy Rust-native assistant platform that can do real work end-to-end, not just demo isolated features.

## v1 Requirements

### Retrieval and Recall

- [ ] **RETR-01**: Recall retrieval combines lexical, vector, recency, confidence, and importance signals instead of relying on a flattened or opaque rank.
- [ ] **RETR-02**: Retrieved memory results are assembled into concise, deduplicated recall output with provenance, freshness, and artifact-type metadata.
- [ ] **RETR-03**: Operators can inspect why a memory was surfaced, including the ranking factors and source artifacts that contributed to the result.
- [ ] **RETR-04**: The runtime keeps the recall-only memory contract by using bounded recall summaries and never injecting raw memory files or raw archive blobs into the system prompt.

### Model Artifacts

- [ ] **MODL-01**: The runtime stores user model, operator model, project memory, and archive summaries as distinct durable artifact classes instead of one blended profile.
- [ ] **MODL-02**: Memory consolidation creates durable summaries and model artifacts only through explicit policy-gated promotion with source lineage.
- [ ] **MODL-03**: The runtime projects only a bounded high-signal subset of structured model artifacts into core memory or prompt context.
- [ ] **MODL-04**: Operators can inspect, correct, deactivate, or remove learned model artifacts when they become stale, wrong, or unsafe.

### Learning Candidates and Lessons

- [ ] **LEAR-01**: Successful runs, reflections, and relevant audit evidence can create durable learning candidates with provenance, confidence, and review state.
- [ ] **LEAR-02**: Learning candidates can be approved, rejected, superseded, or rolled back before they become active lessons or memory artifacts.
- [ ] **LEAR-03**: Promoted lessons can scope guidance for routing, recall, tool choice, or other bounded runtime decisions without silently widening authority.
- [ ] **LEAR-04**: High-impact learned artifacts require replay, evaluation, or equivalent review evidence before promotion.

### Skill Improvement

- [ ] **SKIL-01**: Repeated successful workflows can produce reviewable proposals for new reusable skills or improvements to existing skills.
- [ ] **SKIL-02**: Skill proposals remain human-readable, diffable, and inactive until they pass verification and explicit approval.
- [ ] **SKIL-03**: Approved skill proposals flow through the existing compile and install path with durable provenance back to the source candidate or lesson.

### God Mode

- [ ] **GOD-01**: Operators can explicitly enable a distinct `God Mode` lane that grants full autonomy, full access, and full power.
- [ ] **GOD-02**: God Mode uses explicit scope, TTL or session boundaries, kill-switch controls, and baseline-restore behavior.
- [ ] **GOD-03**: God Mode runs and any learned artifacts they produce are prominently labeled, auditable, and quarantine-capable.
- [ ] **GOD-04**: The default trust-first runtime cannot inherit God Mode permissions, approval bypasses, or tool grants implicitly.

## v2 Requirements

### Advanced Learning

- **ALRN-01**: The runtime can run automatic replay suites continuously to tune lesson usefulness and retrieval quality over time.
- **ALRN-02**: The runtime can compare multiple learning candidates automatically and recommend the highest-yield promotion path.

### Deeper Memory UX

- **MEMX-01**: Operators can browse graph-like relationships between memories, model artifacts, lessons, and skill proposals through a dedicated UI.
- **MEMX-02**: The runtime can support cross-workspace identity stitching for users or operators with explicit governance controls.

### Expanded God Mode Governance

- **GOVR-01**: God Mode can require multiple operators or stronger enterprise approval chains before activation.
- **GOVR-02**: God Mode can run bounded unattended campaigns with explicit objective windows and richer governance policies.

## Out of Scope

| Feature | Reason |
|---------|--------|
| Silent self-editing of installed skills or prompt files | Breaks the trust-first product posture and makes learning behavior too hard to review |
| Raw memory-file injection into every prompt | Violates the existing recall-only memory contract and increases privacy and precision risk |
| A second parallel God Mode runtime stack | Duplicates the existing autonomy lane and increases policy drift risk |
| Broad graph-memory UI or dashboard redesign | Valuable later, but not required to close the core memory and learning gap in this milestone |
| Cross-workspace automatic identity stitching | Too risky for a milestone that first needs trustworthy per-workspace memory and review controls |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| RETR-01 | TBD | Pending |
| RETR-02 | TBD | Pending |
| RETR-03 | TBD | Pending |
| RETR-04 | TBD | Pending |
| MODL-01 | TBD | Pending |
| MODL-02 | TBD | Pending |
| MODL-03 | TBD | Pending |
| MODL-04 | TBD | Pending |
| LEAR-01 | TBD | Pending |
| LEAR-02 | TBD | Pending |
| LEAR-03 | TBD | Pending |
| LEAR-04 | TBD | Pending |
| SKIL-01 | TBD | Pending |
| SKIL-02 | TBD | Pending |
| SKIL-03 | TBD | Pending |
| GOD-01 | TBD | Pending |
| GOD-02 | TBD | Pending |
| GOD-03 | TBD | Pending |
| GOD-04 | TBD | Pending |

**Coverage:**
- v1 requirements: 19 total
- Mapped to phases: 0
- Unmapped: 19

---
*Requirements defined: 2026-04-07*
*Last updated: 2026-04-07 after milestone v1.43 definition*
