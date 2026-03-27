# Requirements: OpenRustClaw

**Defined:** 2026-03-27
**Core Value:** Ship a trustworthy Rust-native assistant platform that can do real work end-to-end, not just demo isolated features.

## v1.7 Requirements

### Canonical Documentation Contract

- [ ] **DOCS-01**: OpenRustClaw defines one canonical ownership contract for `README.md`, repo-root docs mirrors, and the docs-site source so operators know which document is authoritative for each topic.
- [ ] **DOCS-02**: Redundant, stale, or overlapping docs are merged, deleted, or clearly redirected instead of surviving as parallel conflicting sources.

### Product Story and Entry Surfaces

- [ ] **ENTRY-01**: README and docs landing pages present one clear self-hosted open-source product story for `solo`, `team`, `company`, and `enterprise` operators, inspired by OpenClaw-style clarity but grounded in OpenRustClaw’s shipped behavior.
- [ ] **ENTRY-02**: Top-level docs navigation makes it obvious where new users, advanced operators, and enterprise or autonomy-oriented readers should start.

### Setup and Getting Started

- [ ] **SETUP-01**: Installation, quickstart, first-agent, and setup-handoff guidance all describe the same Standard, Advanced, and Custom setup lifecycle plus repair and upgrade or downgrade expectations.

### Operator and Surface Sync

- [ ] **OPS-01**: Deployment, security, observability, and release docs are rewritten to match the shipped runtime, enterprise, and autonomy control surfaces without stale promises.
- [ ] **SURF-01**: Feature-matrix, surface-matrix, product-positioning, and related planning-facing docs are synced with the current shipped product narrative and linked from the canonical entry surfaces.

### Documentation Governance

- [ ] **GOV-01**: The docs set includes an explicit maintenance rule for how future milestones update, merge, delete, and verify canonical docs so drift does not reappear.

## v1.8+ Requirements

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
| Broad new runtime, enterprise, or autonomy implementation work in this milestone | This cycle is for documentation convergence, not another large feature lane |
| A marketing-first site rewrite disconnected from shipped product behavior | The goal is truthful docs, not branding collateral |
| Keeping multiple overlapping copies of the same guide “just in case” | This milestone explicitly aims to reduce drift by deleting or consolidating duplicates |
| Replacing all planning artifacts with polished end-user docs | Planning references still matter, but they need clear boundaries from user-facing guidance |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| DOCS-01 | Phase 32 | Pending |
| DOCS-02 | Phase 32 | Pending |
| ENTRY-01 | Phase 33 | Complete |
| ENTRY-02 | Phase 33 | Complete |
| SETUP-01 | Phase 34 | Pending |
| OPS-01 | Phase 35 | Pending |
| SURF-01 | Phase 35 | Pending |
| GOV-01 | Phase 36 | Pending |

**Coverage:**
- v1.7 requirements: 8 total
- Mapped to phases: 8
- Unmapped: 0

---
*Requirements defined: 2026-03-27*
*Last updated: 2026-03-27 at milestone start*
