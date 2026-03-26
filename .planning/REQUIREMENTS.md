# Requirements: OpenRustClaw

**Defined:** 2026-03-26
**Core Value:** Ship a trustworthy Rust-native assistant platform that can do real work end-to-end, not just demo isolated features.

## v1 Requirements

### Onboarding

- [ ] **ONBD-01**: Operator can install OpenRustClaw and reach a validated local or server deployment path from repository docs and tooling.
- [ ] **ONBD-02**: Operator can run first-start validation that detects missing configuration, providers, secrets, and channel prerequisites before the assistant is exposed to users.
- [ ] **ONBD-03**: Operator can complete first-run setup and reach a working assistant session without manual source edits.

### Assistant Experience

- [ ] **ASST-01**: User can start a primary assistant session and exchange reliable multi-turn chat with the configured model stack.
- [ ] **ASST-02**: User can resume a persisted conversation after restart or reconnect without losing essential session continuity.

### Memory

- [ ] **MEM-01**: Assistant memory persists across restarts and only writes durable facts under an explicit, policy-driven contract.
- [ ] **MEM-02**: Operator can inspect stored memories, recall behavior, and memory-write decisions for debugging and trust.

### Tools and MCP

- [ ] **TOOL-01**: Assistant can discover and execute configured tools or MCP capabilities with clear permission, timeout, and failure behavior.
- [ ] **TOOL-02**: Operator can observe tool execution history and diagnose failed or partial tool runs from runtime artifacts.

### Coding Workflow

- [ ] **CODE-01**: Assistant can inspect, edit, run, and verify code in a bounded coding workflow without leaving the operator in an unrecoverable state.
- [ ] **CODE-02**: Coding actions produce auditable outputs such as transcripts, patches, or execution results that operators can review.

### Communications

- [ ] **COMM-01**: Assistant can send, receive, and act on email workflow tasks through a production-viable integration path.
- [ ] **COMM-02**: Assistant can answer or participate in phone or voice interactions and persist the resulting task or conversation outcome.

### Runtime Operations

- [ ] **OPS-01**: Operator can deploy, start, stop, upgrade, and recover the runtime using a documented and repeatable production path.
- [ ] **OPS-02**: Operator has health, logging, and diagnostics coverage for the assistant, memory, tools, and communication subsystems.

### Security and Release

- [ ] **SEC-01**: Production paths enforce secure-by-default auth, secret handling, origin validation, and sandbox boundaries.
- [ ] **REL-01**: MVP includes an end-to-end verification and release checklist for onboarding, chat, memory, tooling, coding, communications, and runtime ops.

## v2 Requirements

### Enterprise Readiness

- **ENT-01**: Organization can manage multi-tenant deployments with role-based access control and single sign-on.
- **ENT-02**: Platform provides enterprise audit exports, policy controls, and compliance-ready operational evidence.

### Expanded Autonomy

- **AUTO-01**: Assistant can orchestrate longer-running business workflows with stronger approval, rollback, and supervision controls.
- **AUTO-02**: Assistant can manage broader cross-channel automation beyond the MVP trust boundary.

### Surface Expansion

- **SURF-01**: Product reaches deeper parity across all experimental OpenClaw surfaces and edge channels.
- **SURF-02**: Product ships vertical automations such as travel booking and full business back-office workflows.

## Out of Scope

| Feature | Reason |
|---------|--------|
| Fully unsupervised AGI business operator | Beyond the MVP trust, safety, and reliability boundary |
| Enterprise RBAC, SSO, procurement, and compliance packaging | Important, but deferred until the MVP is stable |
| Perfect parity across every OpenClaw lane | Existing breadth should be hardened before every edge surface is carried to production |
| Custom vertical workflows such as flight booking | Build on top of a stable assistant platform after MVP |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| ONBD-01 | Phase 1 | Pending |
| ONBD-02 | Phase 1 | Pending |
| ONBD-03 | Phase 1 | Pending |
| ASST-01 | Phase 2 | Pending |
| ASST-02 | Phase 2 | Pending |
| MEM-01 | Phase 3 | Pending |
| MEM-02 | Phase 3 | Pending |
| TOOL-01 | Phase 4 | Pending |
| TOOL-02 | Phase 4 | Pending |
| CODE-01 | Phase 4 | Pending |
| CODE-02 | Phase 4 | Pending |
| COMM-01 | Phase 5 | Pending |
| COMM-02 | Phase 5 | Pending |
| OPS-01 | Phase 6 | Pending |
| OPS-02 | Phase 6 | Pending |
| SEC-01 | Phase 7 | Pending |
| REL-01 | Phase 7 | Pending |

**Coverage:**
- v1 requirements: 17 total
- Mapped to phases: 17
- Unmapped: 0

---
*Requirements defined: 2026-03-26*
*Last updated: 2026-03-26 after initial definition*
