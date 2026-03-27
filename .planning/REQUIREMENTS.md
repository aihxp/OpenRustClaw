# Requirements: OpenRustClaw

**Defined:** 2026-03-27
**Core Value:** Ship a trustworthy Rust-native assistant platform that can do real work end-to-end, not just demo isolated features.

## v1.8 Requirements

### Cleanup Inventory and Ownership

- [ ] **CLEAN-01**: Maintainer can identify which repo surfaces are canonical, oversized, deprecated, generated, or cleanup candidates before extending them.
- [ ] **CLEAN-02**: Maintainer can see an explicit cleanup target list plus "do not disturb" boundaries for high-risk surfaces before refactoring them.

### Repo Hygiene and Drift Reduction

- [ ] **HYGI-01**: Maintainer can remove stale, duplicate, generated, or local-environment artifacts that do not belong in the canonical product surface.
- [ ] **HYGI-02**: Maintainer can rely on CI, docs, and repo filenames that match the shipped canonical surfaces after cleanup.

### Structural Refactor Boundaries

- [ ] **STRC-01**: Maintainer can work in smaller bounded modules for priority oversized command or control surfaces without changing shipped operator behavior.
- [ ] **STRC-02**: Maintainer can trace cleanup-sensitive contracts between the Rust runtime, optional sidecar, and operator-facing surfaces without hidden coupling.

### Cleanup Safety

- [ ] **SAFE-01**: Maintainer can run a targeted verification bundle that proves cleanup did not regress the shipped onboarding, control, enterprise, autonomy, and docs baselines.
- [ ] **SAFE-02**: Maintainer can find remaining cleanup debt and follow-up boundaries in one maintained artifact instead of rediscovering them ad hoc.

## v1.9+ Requirements

### Deeper Structural Cleanup

- **DEBT-01**: Oversized channel adapters and remaining command hubs can be decomposed further behind stable contracts.
- **DEBT-02**: The Rust runtime and optional compatibility sidecar can reduce protocol overlap and contract drift further.

### Broader Guardrails

- **GUARD-01**: CI can cover a broader matrix of cleanup-sensitive runtime, sidecar, and integration paths.
- **GUARD-02**: Workspace-wide hygiene rules can block accidental local-environment or generated artifact drift earlier.

## Out of Scope

| Feature | Reason |
|---------|--------|
| Broad new product-surface expansion | This milestone is for cleanup and hardening, not another feature-growth wave |
| Deep enterprise RBAC, SSO, SCIM, or tenant lifecycle work | Important, but it should build on a cleaner codebase rather than compete with this cleanup lane |
| New autonomy or parity surfaces | The current need is smaller safer internals, not more breadth |
| Aesthetic-only renames or formatting churn with no cleanup value | Noise would raise review risk without reducing structural debt |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| CLEAN-01 | Phase 37 | Complete |
| CLEAN-02 | Phase 37 | Complete |
| HYGI-01 | Phase 38 | Pending |
| HYGI-02 | Phase 38 | Pending |
| STRC-01 | Phase 39 | Pending |
| STRC-02 | Phase 39 | Pending |
| SAFE-01 | Phase 40 | Pending |
| SAFE-02 | Phase 40 | Pending |

**Coverage:**
- v1.8 requirements: 8 total
- Mapped to phases: 8
- Unmapped: 0

---
*Requirements defined: 2026-03-27*
*Last updated: 2026-03-27 after initial definition*
