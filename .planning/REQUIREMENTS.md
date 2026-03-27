# Requirements: OpenRustClaw

**Defined:** 2026-03-27
**Core Value:** Ship a trustworthy Rust-native assistant platform that can do real work end-to-end, not just demo isolated features.

## Current Status

- No active milestone requirements are open.
- Most recently shipped milestone: `v1.4 Enterprise Governance and Operator-Gated Full Autonomy`
- For archived v1.4 requirements, see `.planning/milestones/v1.4-REQUIREMENTS.md`
- Start the next milestone with `$gsd-new-milestone`

## Recently Validated

- ✓ `GOV-01` and `GOV-02` — stronger enterprise governance and separation-of-duties controls shipped in v1.4 Phase 20
- ✓ `AUD-01` and `AUD-02` — richer enterprise audit retention and review packaging shipped in v1.4 Phase 21
- ✓ `AUTO-05` and `AUTO-06` — explicit operator-gated full autonomy with budgets, kill switch, and durable evidence shipped in v1.4 Phase 22
- ✓ `ADMN-02` — shipped enterprise autonomy control surface in `/control/ui` landed in v1.4 Phase 23

## Next-Line Requirements

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
| Silent or default-on “god mode” for all operators | Full autonomy must remain explicit and operator-gated rather than a hidden weakening of trust boundaries |
| Full unsupervised AGI or autonomous business operation with no operator control path | Still outside the intended trust and safety boundary |
| Full enterprise IAM, SCIM, and multi-tenant governance packaging in one milestone | Still larger than the currently shipped enterprise baseline |
| Another broad multi-surface parity milestone with no enterprise or autonomy guardrails | Future parity work should keep the enterprise and autonomy contract intact |

## Pending Requirement Backlog

| Requirement | Area | Status |
|-------------|------|--------|
| ENT-01 | Enterprise Expansion | Backlog |
| ENT-02 | Enterprise Expansion | Backlog |
| AUTO-01 | Broader Autonomy | Backlog |
| AUTO-02 | Broader Autonomy | Backlog |
| AUTO-03 | Broader Autonomy | Backlog |
| PAR-01 | Additional Parity | Backlog |

**Coverage:**
- Active next-line requirements: 6 total
- Open milestone requirements: 0
- Next milestone: not started

---
*Requirements defined: 2026-03-27*
*Last updated: 2026-03-27 after archiving v1.4*
