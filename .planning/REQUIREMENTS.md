# Requirements: OpenRustClaw

**Defined:** 2026-03-27
**Core Value:** Ship a trustworthy Rust-native assistant platform that can do real work end-to-end, not just demo isolated features.

## v1.3 Requirements

### Enterprise Identity and Access

- [ ] **ENTE-01**: Enterprise operators can authenticate through a first-class organization-oriented access boundary rather than a single shared operator secret.
- [ ] **ENTE-02**: Role and scope boundaries exist for sensitive control-plane actions so approval-sensitive operations are not implicitly all-powerful.

### Enterprise Policy and Audit

- [ ] **ENTE-03**: Enterprise operators can inspect and export durable policy and audit evidence for sensitive assistant actions.
- [ ] **ENTE-04**: Approval, browser, mobile, and autonomy-sensitive policy controls can be configured from one coherent enterprise policy surface.

### Supervised Autonomy Foundations

- [ ] **AUTO-03**: Longer-running supervised workflows expose explicit escalation, pause, rollback, and operator-intervention states instead of opaque execution.
- [ ] **AUTO-04**: Supervised autonomy decisions preserve durable operator-visible evidence about why escalation or rollback happened.

### Admin and Operator Surface

- [ ] **ADMN-01**: The new enterprise and supervised-autonomy capabilities are operator-usable from shipped control surfaces rather than CLI-only internals.

## v1.4+ Requirements

### Broader Enterprise Expansion

- **ENT-01**: Organization can manage multi-tenant deployments with deeper RBAC, SSO lifecycle, and tenant-aware policy controls.
- **ENT-02**: Platform provides compliance-ready exports, retention controls, and broader governance packaging.

### Broader Autonomy

- **AUTO-01**: Assistant can orchestrate longer-running supervised workflows with explicit approval, rollback, and escalation controls across more domains.
- **AUTO-02**: Assistant can manage broader business or operator workflows across multiple channels with stronger supervision.

### Additional Parity

- **PAR-01**: Remaining OpenClaw surfaces can deepen further without weakening the enterprise and autonomy contract.

## Out of Scope

| Feature | Reason |
|---------|--------|
| Full unsupervised AGI or autonomous business operation | Still outside the intended trust and supervision boundary |
| Full compliance packaging, procurement workflows, and regulated-environment certification | Larger than the v1.3 enterprise-foundation slice |
| Another broad multi-surface parity milestone | v1.3 is intentionally enterprise-first, not another wide parity sweep |
| Major new consumer-facing product families unrelated to enterprise and supervised autonomy | Not part of the chosen milestone boundary |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| ENTE-01 | Phase 16 | Pending |
| ENTE-02 | Phase 16 | Pending |
| ENTE-03 | Phase 17 | Pending |
| ENTE-04 | Phase 17 | Pending |
| AUTO-03 | Phase 18 | Pending |
| AUTO-04 | Phase 18 | Pending |
| ADMN-01 | Phase 19 | Pending |

**Coverage:**
- v1.3 requirements: 7 total
- Mapped to phases: 7
- Unmapped: 0

---
*Requirements defined: 2026-03-27*
*Last updated: 2026-03-27 at milestone start*
