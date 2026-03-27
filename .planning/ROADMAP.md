# Roadmap: OpenRustClaw

## Milestones

- ✅ **v1.0 Rust OpenClaw MVP** — shipped 2026-03-26. Archive: `.planning/milestones/v1.0-ROADMAP.md`
- ✅ **v1.1 Lifecycle Integrity and Enterprise Foundations** — shipped 2026-03-26. Archive: `.planning/milestones/v1.1-ROADMAP.md`
- ✅ **v1.2 Deeper OpenClaw Surface Parity** — shipped 2026-03-27. Archive: `.planning/milestones/v1.2-ROADMAP.md`
- 🚧 **v1.3 Enterprise Expansion and Supervised Autonomy Foundations** — phases 16-19

## Roadmap v1.3: Enterprise Expansion and Supervised Autonomy Foundations

### Overview

This milestone expands the platform toward enterprise readiness without losing the trust-first baseline already shipped. The order is deliberate: first establish enterprise identity and access boundaries, then deepen policy and audit controls, then add the supervised-autonomy lifecycle those controls need, and finally close the slice with one enabling admin/operator surface.

### Phases

**Phase Numbering:**
- Integer phases continue across milestones to preserve one linear execution history.
- Decimal phases are reserved for urgent insertions if roadmap assumptions break.

- [ ] **Phase 16: Enterprise Identity and Access Boundaries** - Add organization-oriented authentication and role-aware control boundaries for sensitive operator actions.
- [ ] **Phase 17: Enterprise Policy and Audit Controls** - Expand policy management and audit evidence into a real enterprise operator surface.
- [ ] **Phase 18: Supervised Autonomy Escalation and Rollback** - Add explicit escalation, rollback, and operator-intervention semantics for longer-running supervised workflows.
- [ ] **Phase 19: Enterprise Admin Surface** - Make the new enterprise and supervised-autonomy controls usable from shipped operator surfaces.

### Phase Details

### Phase 16: Enterprise Identity and Access Boundaries
**Goal**: Add enterprise-oriented authentication and role-aware access boundaries without weakening the shipped control-plane trust model.
**Depends on**: v1.2 archive state
**Requirements**: [ENTE-01, ENTE-02]
**Success Criteria** (what must be TRUE):
  1. Sensitive operator actions no longer assume one shared operator identity.
  2. Role and scope boundaries exist for enterprise-sensitive control-plane actions.
  3. The new access model remains inspectable and compatible with the existing Rust-owned control plane.
**Plans**: TBD

Plans:
- [ ] TBD (run `$gsd-plan-phase 16` to break down)

### Phase 17: Enterprise Policy and Audit Controls
**Goal**: Expand policy and audit surfaces so enterprise operators can inspect, configure, and export sensitive-action evidence coherently.
**Depends on**: Phase 16
**Requirements**: [ENTE-03, ENTE-04]
**Success Criteria** (what must be TRUE):
  1. Enterprise operators can inspect durable policy and audit state from one coherent surface.
  2. Approval-sensitive and autonomy-sensitive controls are explicitly configurable rather than hardcoded assumptions.
  3. Exportable evidence exists for the new enterprise policy and audit contract.
**Plans**: TBD

Plans:
- [ ] TBD (run `$gsd-plan-phase 17` to break down)

### Phase 18: Supervised Autonomy Escalation and Rollback
**Goal**: Add explicit escalation, rollback, and intervention semantics for longer-running supervised workflows.
**Depends on**: Phase 17
**Requirements**: [AUTO-03, AUTO-04]
**Success Criteria** (what must be TRUE):
  1. Longer-running supervised workflows can escalate, pause, and roll back through explicit lifecycle states.
  2. Escalation and rollback decisions preserve durable operator-visible evidence.
  3. The autonomy expansion remains supervision-first rather than drifting toward opaque autonomous execution.
**Plans**: TBD

Plans:
- [ ] TBD (run `$gsd-plan-phase 18` to break down)

### Phase 19: Enterprise Admin Surface
**Goal**: Make the new enterprise and supervised-autonomy capabilities operator-usable from shipped control surfaces.
**Depends on**: Phase 18
**Requirements**: [ADMN-01]
**Success Criteria** (what must be TRUE):
  1. Operators can use the new enterprise and supervised-autonomy controls without falling back to scattered CLI internals.
  2. The admin/operator surface stays grounded in typed runtime contracts rather than frontend-only stitching.
  3. Docs and verification close the milestone with a truthful enterprise-ready baseline.
**Plans**: TBD

Plans:
- [ ] TBD (run `$gsd-plan-phase 19` to break down)

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 16. Enterprise Identity and Access Boundaries | 0/TBD | Not started | - |
| 17. Enterprise Policy and Audit Controls | 0/TBD | Not started | - |
| 18. Supervised Autonomy Escalation and Rollback | 0/TBD | Not started | - |
| 19. Enterprise Admin Surface | 0/TBD | Not started | - |

## Current Status

- Active milestone: v1.3 Enterprise Expansion and Supervised Autonomy Foundations
- Next step: `$gsd-discuss-phase 16` or `$gsd-plan-phase 16`
- Archived milestone planning artifacts live under `.planning/milestones/`.
