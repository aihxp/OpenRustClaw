# Requirements: OpenRustClaw

**Defined:** 2026-03-27
**Core Value:** Ship a trustworthy Rust-native assistant platform that can do real work end-to-end, not just demo isolated features.

## v1.5 Requirements

### Self-Hosted Product Modes

- [ ] **MODE-01**: OpenRustClaw is framed explicitly as a self-hosted open-source product for solo, multi-user team, company, and enterprise deployments.
- [ ] **MODE-02**: Deployment mode is modeled as a first-class instance profile rather than implied by scattered docs or enterprise toggles.

### Tiered Onboarding

- [ ] **ONBR-01**: First-run setup offers differentiated paths for solo, multi-user team, company, and enterprise installs instead of one flat onboarding flow.
- [ ] **ONBR-02**: Each onboarding path explains the mode-specific trust boundary, required setup steps, and recommended defaults truthfully.

### Upgrade and Downgrade Lifecycle

- [ ] **LIFE-01**: Operators can upgrade between product modes without ambiguous state transitions or hidden policy drift.
- [ ] **LIFE-02**: Operators can downgrade between product modes with explicit handling for permissions, governance, and retained data implications.

### Product and Operator Surface Alignment

- [ ] **SURF-01**: Docs and shipped control surfaces consistently describe the product as self-hosted open-source and reflect the current deployment mode plus transition path.

## v1.6+ Requirements

### Broader Enterprise Expansion

- [ ] **ENT-01**: Organization can manage multi-tenant deployments with deeper RBAC, SSO lifecycle, and tenant-aware policy controls.
- [ ] **ENT-02**: Platform provides compliance-ready exports, retention controls, and broader governance packaging.

### Broader Autonomy

- [ ] **AUTO-01**: Assistant can orchestrate longer-running supervised workflows with explicit approval, rollback, and escalation controls across more domains.
- [ ] **AUTO-02**: Assistant can manage broader business or operator workflows across multiple channels with stronger supervision.
- [ ] **AUTO-03**: Full-autonomy mode can expand into more domains without dropping kill switches, budgets, or durable evidence.

### Additional Parity

- [ ] **PAR-01**: Remaining OpenClaw surfaces can deepen further without weakening the enterprise and autonomy contract.

## Out of Scope

| Feature | Reason |
|---------|--------|
| Hosted SaaS packaging, billing, or commercial licensing workflows in this milestone | The current request is about self-hosted mode clarity and lifecycle, not hosted commercialization |
| Full enterprise IAM, SCIM, and tenant-aware RBAC implementation in the same milestone | Larger than the focused product-mode and onboarding slice |
| Automatic in-place migrations for every future mode change with no operator review | Upgrade and downgrade paths should be explicit and operator-auditable first |
| Silent or default-on full-autonomy mode for all installs | The self-hosted mode system should not weaken the operator-gated trust contract already established |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| MODE-01 | Phase 24 | Pending |
| MODE-02 | Phase 24 | Pending |
| ONBR-01 | Phase 25 | Pending |
| ONBR-02 | Phase 25 | Pending |
| LIFE-01 | Phase 26 | Pending |
| LIFE-02 | Phase 26 | Pending |
| SURF-01 | Phase 27 | Pending |

**Coverage:**
- v1.5 requirements: 7 total
- Mapped to phases: 7
- Unmapped: 0

---
*Requirements defined: 2026-03-27*
*Last updated: 2026-03-27 at milestone start*
