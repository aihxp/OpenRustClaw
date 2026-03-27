# Requirements: OpenRustClaw

**Defined:** 2026-03-28
**Core Value:** Ship a trustworthy Rust-native assistant platform that can do real work end-to-end, not just demo isolated features.

## v1.6 Requirements

### Setup State and Recovery

- [x] **SETUP-01**: Onboarding persists a resumable setup-state contract that records deployment path, completed steps, current blockers, and the next recommended action.
- [x] **SETUP-02**: Existing or partially configured workspaces can re-enter setup through explicit resume, repair, or reset-with-backup paths instead of manual state edits.

### Setup Path Selection

- [x] **PATH-01**: First-run setup offers a standard path for the common self-hosted install and an advanced or custom path for operators who need deeper control.
- [x] **PATH-02**: Standard and advanced or custom setup paths stay mode-aware and converge back into the same truthful readiness and handoff contract.

### Provider, Runtime, and Channel Bootstrap

- [x] **BOOT-01**: Setup drives the core provider, model, runtime, and control-plane configuration to a truthful ready or blocked state for the selected deployment mode.
- [x] **BOOT-02**: Setup can validate and, where supported, bootstrap key runtime or channel surfaces before claiming the workspace is ready.

### Operator Handoff and Visibility

- [ ] **HANDOFF-01**: Setup ends with an explicit ready, blocked, or degraded handoff summary rather than a vague success message.
- [ ] **HANDOFF-02**: Docs and shipped operator surfaces consistently reflect setup progress, unresolved blockers, and the next action after onboarding.

## v1.7+ Requirements

### Broader Enterprise Expansion

- [ ] **ENT-01**: Organization can manage multi-tenant deployments with deeper RBAC, SSO lifecycle, and tenant-aware policy controls.
- [ ] **ENT-02**: Platform provides compliance-ready exports, retention controls, and broader governance packaging.

### Broader Autonomy

- [ ] **AUTO-01**: Assistant can orchestrate longer-running supervised workflows with explicit approval, rollback, and escalation controls across more domains.
- [ ] **AUTO-02**: Assistant can manage broader business or operator workflows across multiple channels with stronger supervision.
- [ ] **AUTO-03**: Full-autonomy mode can expand into more domains without dropping kill switches, budgets, or durable evidence.

### Additional Parity

- [ ] **PAR-01**: Remaining OpenClaw surfaces can deepen further without weakening the enterprise, autonomy, and self-hosted product contract.

## Out of Scope

| Feature | Reason |
|---------|--------|
| Hosted SaaS onboarding, billing, or commercial account packaging in this milestone | The goal is to improve self-hosted onboarding and setup, not add a hosted product |
| Deep enterprise IAM, SCIM, or tenant-aware RBAC implementation in the same milestone | Larger than the setup hardening lane |
| Full autonomous setup that mutates all runtime surfaces without operator review | Setup still needs explicit trust boundaries and truthful readiness checks |
| Broad new assistant or channel feature families unrelated to onboarding or setup | This milestone is convergence work on the first-install path |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| SETUP-01 | Phase 28 | Complete |
| PATH-01 | Phase 28 | Complete |
| PATH-02 | Phase 29 | Complete |
| SETUP-02 | Phase 30 | Complete |
| BOOT-01 | Phase 29 | Complete |
| BOOT-02 | Phase 29 | Complete |
| HANDOFF-01 | Phase 31 | Pending |
| HANDOFF-02 | Phase 31 | Pending |

**Coverage:**
- v1.6 requirements: 8 total
- Mapped to phases: 8
- Unmapped: 0

---
*Requirements defined: 2026-03-28*
*Last updated: 2026-03-28 at milestone start*
