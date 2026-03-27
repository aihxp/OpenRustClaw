# Requirements: OpenRustClaw

**Defined:** 2026-03-27
**Core Value:** Ship a trustworthy Rust-native assistant platform that can do real work end-to-end, not just demo isolated features.

## v1.4 Requirements

### Enterprise Governance

- [ ] **GOV-01**: Enterprise operators can define stronger role and approval-chain boundaries for sensitive runtime, mobile, browser, and autonomy actions.
- [ ] **GOV-02**: Enterprise governance surfaces preserve separation of duties so high-risk actions are not implicitly granted by one broad operator role.

### Enterprise Audit and Review

- [ ] **AUD-01**: Enterprise operators can retain and export richer governance evidence bundles with policy context, operator history, and autonomy-related evidence.
- [ ] **AUD-02**: Enterprise audit and retention controls are configurable from one coherent enterprise governance surface.

### Operator-Gated Full Autonomy

- [ ] **AUTO-05**: Trusted operators can explicitly enable a separate full-autonomy mode without changing the default trust-first runtime contract for everyone else.
- [ ] **AUTO-06**: Full-autonomy runs preserve explicit budgets, kill switches, and durable evidence about enablement, execution, and shutdown.

### Admin and Control Surface

- [ ] **ADMN-02**: Enterprise operators can inspect, configure, and shut down the full-autonomy override lane from shipped control surfaces rather than CLI-only internals.

## v1.5+ Requirements

### Broader Enterprise Expansion

- **ENT-01**: Organization can manage multi-tenant deployments with deeper RBAC, SSO lifecycle, and tenant-aware policy controls.
- **ENT-02**: Platform provides compliance-ready exports, retention controls, and broader governance packaging.

### Broader Autonomy

- **AUTO-01**: Assistant can orchestrate longer-running supervised workflows with explicit approval, rollback, and escalation controls across more domains.
- **AUTO-02**: Assistant can manage broader business or operator workflows across multiple channels with stronger supervision.
- **AUTO-03**: Full-autonomy mode can expand into more domains without dropping kill switches, budgets, or durable evidence.

### Additional Parity

- **PAR-01**: Remaining OpenClaw surfaces can deepen further without weakening the enterprise and autonomy contract.

## Out of Scope

| Feature | Reason |
|---------|--------|
| Silent or default-on “god mode” for all operators | Full autonomy must be explicit and operator-gated rather than a hidden weakening of trust boundaries |
| Full unsupervised AGI or autonomous business operation with no operator control path | Still outside the intended trust and safety boundary |
| Full enterprise IAM, SCIM, and multi-tenant governance packaging in one milestone | Larger than the focused v1.4 enterprise-governance slice |
| Another broad multi-surface parity milestone | v1.4 is enterprise-first with one autonomy lane, not another wide parity sweep |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| GOV-01 | Phase 20 | Pending |
| GOV-02 | Phase 20 | Pending |
| AUD-01 | Phase 21 | Pending |
| AUD-02 | Phase 21 | Pending |
| AUTO-05 | Phase 22 | Pending |
| AUTO-06 | Phase 22 | Pending |
| ADMN-02 | Phase 23 | Pending |

**Coverage:**
- v1.4 requirements: 7 total
- Mapped to phases: 7
- Unmapped: 0

---
*Requirements defined: 2026-03-27*
*Last updated: 2026-03-27 at milestone start*
