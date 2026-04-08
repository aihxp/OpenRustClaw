# Requirements: OpenRustClaw v1.45 Agent Fabric, Routing Console, and Guided Delegation

**Defined:** 2026-04-08
**Core Value:** Ship a trustworthy Rust-native assistant platform that can do real work end-to-end, not just demo isolated features.

## v1 Requirements

### Cursor and Backend Expansion

- [ ] **CURS-01**: Operator can see whether Cursor exposes a documented programmable execution surface and why it is supported, blocked, or detection-only.
- [ ] **CURS-02**: If Cursor exposes a supported execution surface, OpenRustClaw can route bounded delegated work through it with the same audit, sandbox, and policy controls used for other delegated backends.
- [ ] **CURS-03**: If Cursor does not expose a supported execution surface, onboarding, models, inspect, and Control UI all keep Cursor visible as detection-only with explicit compliance reasons.

### Delegation Fabric

- [ ] **FABR-01**: Operator can register and inspect trusted remote hosts that advertise delegated agent backends and their capability metadata.
- [ ] **FABR-02**: Route selection can evaluate both local and trusted remote delegated backends using explicit readiness, policy, and compatibility signals.
- [x] **FABR-03**: Remote delegated execution preserves audit evidence, operator attribution, environment allowlists, and bounded runtime controls.
- [x] **FABR-04**: When a delegated backend is blocked or fails, OpenRustClaw records the route decision and recovery hints instead of failing silently.

### Routing Console

- [ ] **ROUTX-01**: Control UI exposes a dedicated routing console that shows delegated backend inventory, readiness, policy state, and routeable capacity.
- [ ] **ROUTX-02**: Operator can enable, disable, or constrain delegated backends and route policy through shipped CLI and Control UI surfaces without editing raw files.
- [ ] **ROUTX-03**: Operators can inspect route decisions and delegated receipts per task or run using the same vocabulary across CLI, Control UI, and inspect surfaces.

### Guided First-Task Orchestration

- [ ] **TASK-01**: The first task after onboarding or repair suggests a truthful execution path based on the selected lane, available backends, and current policy state.
- [ ] **TASK-02**: Orchestration can recommend or prefill the right provider, delegated backend, claw, or model-profile path for an initial task instead of dropping operators into generic defaults.
- [ ] **TASK-03**: If the preferred lane is blocked, unavailable, or detection-only, OpenRustClaw offers actionable fallback choices before first-task execution begins.

## v2 Requirements

### Delegation Fabric Depth

- **FABR-05**: Delegated backend routing can weight latency, quotas, and spend hints across multiple hosts.
- **FABR-06**: Operator can group remote backends into pools and assign workload classes to those pools.

### Routing UX Depth

- **ROUTX-04**: Control UI visualizes live route saturation, queue depth, and backend health trends over time.
- **ROUTX-05**: Operators can simulate a route decision before sending a task.

### Guided Task UX Depth

- **TASK-04**: OpenRustClaw can turn the onboarding handoff into a one-click “run the first meaningful task” journey with preflight and rollback baked in.

## Out of Scope

| Feature | Reason |
|---------|--------|
| Importing vendor browser sessions, cached OAuth tokens, or desktop secrets from Cursor, Claude Code, Codex, or Gemini CLI | Violates the trust-first and compliance-safe backend contract |
| Pretending Cursor is a supported backend without a documented programmable surface | Detection is useful, but fake support would reopen the exact truthfulness gap v1.44 just closed |
| Fully automatic ambient discovery of remote hosts without explicit operator trust enrollment | Multi-host delegation needs a clear trust and audit boundary |
| Broad visual redesign work unrelated to routing clarity, backend policy, or first-task guidance | Valuable later, but not required to close this milestone’s product-shape gap |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| CURS-01 | Phase 191 | Complete |
| CURS-02 | Phase 191 | Complete |
| CURS-03 | Phase 191 | Complete |
| FABR-01 | Phase 192 | Complete |
| FABR-02 | Phase 192 | Complete |
| FABR-03 | Phase 193 | Complete |
| FABR-04 | Phase 193 | Complete |
| ROUTX-01 | Phase 194 | Pending |
| ROUTX-02 | Phase 194 | Pending |
| ROUTX-03 | Phase 194 | Pending |
| TASK-01 | Phase 195 | Pending |
| TASK-02 | Phase 195 | Pending |
| TASK-03 | Phase 195 | Pending |

**Coverage:**
- v1 requirements: 13 total
- Mapped to phases: 13
- Unmapped: 0

---
*Requirements defined: 2026-04-08*
*Last updated: 2026-04-08 after initial definition*
