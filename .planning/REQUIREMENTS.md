# Requirements: OpenRustClaw

**Defined:** 2026-03-26
**Core Value:** Ship a trustworthy Rust-native assistant platform that can do real work end-to-end, not just demo isolated features.

## v1.2 Requirements

### Browser Parity

- [x] **BROW-01**: Operator can run deeper multi-step browser workflows with durable artifacts and inspection surfaces that feel closer to OpenClaw’s browser lane.
- [x] **BROW-02**: Browser automation depth remains policy-bounded and auditable instead of bypassing the existing external-backend trust contract.

### Supervision Parity

- [x] **SUPR-01**: Operator can inspect and control multi-agent or orchestration runs with stronger supervision detail, not just final receipts.
- [x] **SUPR-02**: Multi-agent parity surfaces preserve explicit approval, trace, and resource visibility when delegated work expands.

### Mobile Parity

- [ ] **MOBL-01**: Operator can inspect a broader mobile runtime state and action surface that feels materially closer to OpenClaw’s mobile lane.
- [ ] **MOBL-02**: Mobile parity expansions keep approval gates, receipts, and operator evidence first-class.

### Control UI Parity

- [ ] **CTRL-01**: Control UI exposes deeper parity across the shipped runtime surfaces so operators do not need to drop to scattered raw endpoints for common workflows.
- [ ] **CTRL-02**: New Control UI parity surfaces remain coherent with the typed runtime contracts rather than adding one-off frontend-only logic.

### Voice and Call Parity

- [ ] **VOIC-01**: Operator can inspect and manage richer voice or call-handling workflows that move closer to OpenClaw’s real-time voice surface.
- [ ] **VOIC-02**: Voice and call parity surfaces preserve durable artifacts, session health, and operator-visible evidence instead of opaque runtime behavior.

## v1.3+ Requirements

### Enterprise Expansion

- **ENT-01**: Organization can manage multi-tenant deployments with role-based access control and single sign-on.
- **ENT-02**: Platform provides enterprise audit exports, policy controls, and compliance-ready operational evidence.

### Broader Autonomy

- **AUTO-01**: Assistant can orchestrate longer-running supervised workflows with explicit approval, rollback, and escalation controls.
- **AUTO-02**: Assistant can manage broader business or operator workflows across multiple channels with stronger supervision.

## Out of Scope

| Feature | Reason |
|---------|--------|
| Full enterprise RBAC, SSO, multi-tenancy, and compliance packaging | Still broader than the focused parity slice for v1.2 |
| Full unsupervised AGI or autonomous business operation | Trust and supervision boundaries are still intentionally explicit |
| Simultaneous parity across every OpenClaw surface | The milestone is intentionally narrowed to the top five parity surfaces |
| New unrelated vertical automations like flight booking or full business back-office orchestration | Not part of the chosen parity-first milestone |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| BROW-01 | Phase 11 | Complete |
| BROW-02 | Phase 11 | Complete |
| SUPR-01 | Phase 12 | Complete |
| SUPR-02 | Phase 12 | Complete |
| MOBL-01 | Phase 13 | Pending |
| MOBL-02 | Phase 13 | Pending |
| CTRL-01 | Phase 14 | Pending |
| CTRL-02 | Phase 14 | Pending |
| VOIC-01 | Phase 15 | Pending |
| VOIC-02 | Phase 15 | Pending |

**Coverage:**
- v1.2 requirements: 10 total
- Mapped to phases: 10
- Unmapped: 0

---
*Requirements defined: 2026-03-26*
*Last updated: 2026-03-27 after Phase 12 completion*
