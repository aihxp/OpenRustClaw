# Requirements: OpenRustClaw

**Defined:** 2026-03-26
**Core Value:** Ship a trustworthy Rust-native assistant platform that can do real work end-to-end, not just demo isolated features.

## v1.1 Requirements

### Lifecycle Integrity

- [x] **LIFE-01**: Operator can rely on every completed phase producing a structured `VERIFICATION.md` artifact with phase goal, requirement coverage, evidence, and final status.
- [x] **LIFE-02**: Operator cannot advance phase-complete, milestone-audit, or milestone-archive flows when required verification artifacts are missing or stale.
- [ ] **LIFE-03**: Operator can review archived milestone verification evidence and any accepted verification debt from shipped planning artifacts without reconstructing history manually.

### Enterprise Foundations

- [ ] **ENTF-01**: Operator can inspect a durable audit trail for enterprise-sensitive assistant actions from shipped control or runtime surfaces.
- [ ] **ENTF-02**: Enterprise-sensitive assistant actions execute under an explicit approval or policy contract instead of implicit best-effort behavior.
- [ ] **ENTF-03**: Operator-facing docs and inspection surfaces explain the enterprise approval and audit baseline clearly enough for the next enterprise milestone to build on it.

## v1.2+ Requirements

### Enterprise Expansion

- **ENT-01**: Organization can manage multi-tenant deployments with role-based access control and single sign-on.
- **ENT-02**: Platform provides enterprise audit exports, policy controls, and compliance-ready operational evidence.

### Broader Autonomy

- **AUTO-01**: Assistant can orchestrate longer-running supervised workflows with explicit approval, rollback, and escalation controls.
- **AUTO-02**: Assistant can manage broader business or operator workflows across multiple channels with stronger supervision.

### Deeper Surface Parity

- **PAR-01**: Product reaches deeper parity across more OpenClaw surfaces without regressing the shipped trust baseline.
- **PAR-02**: Product expands vertical automations only after the lifecycle and enterprise foundations are solid.

## Out of Scope

| Feature | Reason |
|---------|--------|
| Full enterprise RBAC, SSO, multi-tenancy, and compliance packaging | Too broad for the first post-MVP milestone; foundations first |
| Broad autonomy or business-operator orchestration | Needs stronger approval, rollback, and policy foundations first |
| Deep parity across every OpenClaw surface | Would reopen MVP sprawl before lifecycle integrity is fixed |
| A retrospective backfill of every missing v1.0 verification report | The milestone should fix the workflow contract going forward before attempting large historical reconstruction |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| LIFE-01 | Phase 8 | Complete |
| LIFE-02 | Phase 8 | Complete |
| LIFE-03 | Phase 9 | Pending |
| ENTF-01 | Phase 10 | Pending |
| ENTF-02 | Phase 10 | Pending |
| ENTF-03 | Phase 10 | Pending |

**Coverage:**
- v1.1 requirements: 6 total
- Mapped to phases: 6
- Unmapped: 0

---
*Requirements defined: 2026-03-26*
*Last updated: 2026-03-26 after v1.1 initial definition*
