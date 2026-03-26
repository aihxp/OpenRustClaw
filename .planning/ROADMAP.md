# Roadmap: OpenRustClaw

## Milestones

- ✅ **v1.0 Rust OpenClaw MVP** — shipped 2026-03-26. Archive: `.planning/milestones/v1.0-ROADMAP.md`
- 🚧 **v1.1 Lifecycle Integrity and Enterprise Foundations** — phases 8-10

## Roadmap v1.1: Lifecycle Integrity and Enterprise Foundations

### Overview

This milestone fixes the lifecycle evidence debt exposed by v1.0 before the project expands aggressively again. The order is deliberate: first make per-phase verification artifacts unavoidable, then harden milestone audit and archive integrity around those artifacts, and only then carry a narrow enterprise-readiness slice around approval and audit foundations.

### Phases

**Phase Numbering:**
- Integer phases continue across milestones to preserve one linear execution history.
- Decimal phases are reserved for urgent insertions if roadmap assumptions break.

- [ ] **Phase 8: Verification Artifact Contract** - Make phase execution produce durable verification artifacts by default and block false completion when they are missing.
- [ ] **Phase 9: Milestone Lifecycle Integrity** - Make audit, archive, and cleanup consume and preserve verification evidence honestly.
- [ ] **Phase 10: Enterprise Policy and Audit Foundations** - Start enterprise readiness with explicit approval boundaries and durable audit evidence for sensitive assistant actions.

### Phase Details

### Phase 8: Verification Artifact Contract
**Goal**: Every completed phase must produce a truthful verification artifact, and phase progression must fail fast when that evidence is absent.
**Depends on**: v1.0 archive state
**Requirements**: [LIFE-01, LIFE-02]
**Success Criteria** (what must be TRUE):
  1. Completed phases always emit a structured `VERIFICATION.md` with goal, requirements coverage, evidence, and result.
  2. Phase execution, progress updates, and autonomous flow do not mark phases complete when required verification artifacts are missing or stale.
  3. Operators can see verification readiness gaps before milestone lifecycle steps begin.
**Plans**: TBD

Plans:
- [ ] TBD (run `$gsd-plan-phase 8` to break down)

### Phase 9: Milestone Lifecycle Integrity
**Goal**: Milestone audit, completion, and cleanup must preserve and surface verification evidence instead of relying on reconstruction after the fact.
**Depends on**: Phase 8
**Requirements**: [LIFE-03]
**Success Criteria** (what must be TRUE):
  1. Milestone audit consumes the expected verification artifacts and reports missing evidence as a first-class result.
  2. Milestone archive outputs preserve verification evidence and known verification debt in the shipped planning record.
  3. Cleanup and archive flows do not silently discard verification artifacts needed for later review.
**Plans**: TBD

Plans:
- [ ] TBD (run `$gsd-plan-phase 9` to break down)

### Phase 10: Enterprise Policy and Audit Foundations
**Goal**: Establish a narrow enterprise baseline for approval-sensitive actions and durable audit evidence without attempting full enterprise scope.
**Depends on**: Phase 9
**Requirements**: [ENTF-01, ENTF-02, ENTF-03]
**Success Criteria** (what must be TRUE):
  1. Sensitive assistant actions execute under an explicit approval or policy boundary rather than implicit operator trust.
  2. Operators can inspect a durable audit trail for those sensitive actions from shipped surfaces.
  3. Docs and operator-facing inspection surfaces explain the enterprise baseline clearly enough to support the next milestone.
**Plans**: TBD

Plans:
- [ ] TBD (run `$gsd-plan-phase 10` to break down)

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 8. Verification Artifact Contract | 0/TBD | Not started | - |
| 9. Milestone Lifecycle Integrity | 0/TBD | Not started | - |
| 10. Enterprise Policy and Audit Foundations | 0/TBD | Not started | - |
