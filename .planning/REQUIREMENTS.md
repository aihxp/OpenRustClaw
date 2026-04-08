# Requirements: OpenRustClaw v1.44 Agent Discovery, Journey Cohesion, and Provider Access

**Defined:** 2026-04-08
**Core Value:** Ship a trustworthy Rust-native assistant platform that can do real work end-to-end, not just demo isolated features.

## v1 Requirements

### Local Agent Discovery and Compliance

- [x] **DISC-01**: OpenRustClaw detects supported local agent tools such as `claude`, `codex`, `gemini`, and any documented Cursor agent surface without unsafe probing or hidden side effects.
- [x] **DISC-02**: Discovery records durable evidence for each detected backend, including binary path, version, invocation style, auth status or auth capability, model-discovery capability, and policy classification.
- [x] **DISC-03**: Detected-but-unusable agent tools stay visible with truthful reasons such as unsupported auth reuse, missing model discovery, blocked policy, or missing vendor support.
- [x] **DISC-04**: Local agent discovery and refresh actions are policy-gated, auditable, and available through shared app services instead of disconnected CLI-only checks.

### OAuth-Safe Account-Backed Access

- [x] **AUTH-01**: Subscription-managed or account-managed access is only used through supported vendor execution surfaces or documented APIs; OpenRustClaw must not import cached browser sessions, copy tokens from vendor stores, or impersonate vendor logins.
- [x] **AUTH-02**: OpenRustClaw distinguishes direct API providers from delegated local agent backends and preserves truthful capability metadata for each lane.
- [x] **AUTH-03**: Backend policy can allow or deny delegated local agent execution per backend while preserving environment allowlists, audit logging, and runtime controls.
- [x] **AUTH-04**: Model availability for delegated agent backends is discovered truthfully when supported and otherwise labeled as vendor-managed or unknown instead of guessed.

### Onboarding and Model Selection Cohesion

- [ ] **ONBR-01**: Onboarding surfaces detected local agent backends alongside API-key and local-runtime providers with clear access-mode labels and compatibility notes.
- [ ] **ONBR-02**: Provider, access-mode, and model selection persist delegated-agent choices and restore them through handoff, repair, resume, and first launch.
- [ ] **ONBR-03**: Onboarding explains vendor constraints and fallback options whenever a detected local agent backend cannot safely serve as a general-purpose provider lane.
- [ ] **ONBR-04**: `openrustclaw models`, onboarding menus, inspect surfaces, and control surfaces share one canonical provider or agent catalog instead of diverging static lists.

### Runtime Integration and Agent Journey

- [ ] **ROUT-01**: Eligible tasks can route through installed local agent backends as bounded external backends with durable audit evidence and consistent session attribution.
- [ ] **ROUT-02**: Model profiles and control-registry flows can reference delegated agent backends without breaking existing provider traits, fallback chains, or runtime mode controls.
- [ ] **ROUT-03**: Delegated agent execution preserves approval, autonomy, memory, and artifact boundaries instead of widening authority implicitly.
- [ ] **ROUT-04**: Operators can inspect backend selection, execution receipts, failures, and recovery hints for delegated local-agent runs.

### Journey Audit and UX Repair

- [ ] **JOUR-01**: The user journey from install to onboarding to first task to inspect or repair contains no dead-end steps or contradictory terminology around providers, agents, or access modes.
- [ ] **JOUR-02**: The agent journey from discovery to selection to routing to result to inspection is explicit and testable across CLI, Control UI, and MCP surfaces.
- [ ] **JOUR-03**: Disconnected provider catalogs, model menus, policy surfaces, and inspection reports are converged behind shared app services or typed contracts.
- [ ] **JOUR-04**: Docs, onboarding text, inspect output, and control UI tell the same truthful story about supported local agents, provider access, and delegated execution.

## v2 Requirements

### Multi-Host Agent Fabric

- **FABR-01**: OpenRustClaw can discover and manage trusted agent backends across multiple hosts instead of one local machine.
- **FABR-02**: Delegated agent pools can advertise quotas, concurrency, and priority routing to the orchestrator.

### Billing and Usage Awareness

- **BILL-01**: Delegated agent execution can surface vendor-specific quota or billing hints where the vendor exposes them safely.
- **BILL-02**: Operators can enforce per-backend spend or usage ceilings before launching delegated work.

### Deeper UX Surfaces

- **UX-01**: Control UI can visualize the full backend discovery graph, policy state, and route decisions in one operator-facing panel.
- **UX-02**: Onboarding can run a guided compatibility audit for all detected agent tools before the first model selection decision.

## Out of Scope

| Feature | Reason |
|---------|--------|
| Importing vendor session cookies, OAuth tokens, or credential-store secrets into OpenRustClaw | Breaks the trust-first posture and is not a documented vendor integration path |
| Treating an installed desktop launcher as a full model backend without a documented execution surface | Detection is useful, but pretending capability would make onboarding dishonest |
| Using consumer subscriptions as raw API replacements when vendor docs require direct API or cloud credentials | Risks violating vendor boundaries and produces fragile integrations |
| Silent background delegation to external agent CLIs without audit evidence or operator inspection | Conflicts with the existing bounded-autonomy contract |
| Broad visual redesign work unrelated to journey coherence or agent usability | Valuable later, but not required to close the current product truthfulness and UX gaps |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| DISC-01 | Phase 186 | Completed |
| DISC-02 | Phase 186 | Completed |
| DISC-03 | Phase 186 | Completed |
| DISC-04 | Phase 186 | Completed |
| AUTH-01 | Phase 187 | Completed |
| AUTH-02 | Phase 187 | Completed |
| AUTH-03 | Phase 187 | Completed |
| AUTH-04 | Phase 187 | Completed |
| ONBR-01 | Phase 188 | Planned |
| ONBR-02 | Phase 188 | Planned |
| ONBR-03 | Phase 188 | Planned |
| ONBR-04 | Phase 188 | Planned |
| ROUT-01 | Phase 189 | Planned |
| ROUT-02 | Phase 189 | Planned |
| ROUT-03 | Phase 189 | Planned |
| ROUT-04 | Phase 189 | Planned |
| JOUR-01 | Phase 190 | Planned |
| JOUR-02 | Phase 190 | Planned |
| JOUR-03 | Phase 190 | Planned |
| JOUR-04 | Phase 190 | Planned |

**Coverage:**
- v1 requirements: 20 total
- Mapped to phases: 20
- Unmapped: 0

---
*Requirements defined: 2026-04-08*
*Last updated: 2026-04-08 after completing Phase 187*
