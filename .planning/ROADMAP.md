# Roadmap: OpenRustClaw

## Roadmap v1.0: Rust OpenClaw MVP

### Overview

This milestone turns the existing OpenRustClaw surface area into a believable production-ready MVP. The order is deliberate: first remove onboarding friction, then stabilize the assistant session, then lock down memory behavior, then harden tools and coding flows, then carry communications into a shippable state, then make runtime operations dependable, and finally close the milestone with security and release exit criteria.

### Phases

**Phase Numbering:**
- Integer phases are planned milestone work.
- Decimal phases are reserved for urgent insertions if roadmap assumptions break.

- [ ] **Phase 1: Onboarding and First-Run Trust** - Make install, configuration, and first assistant launch production-ready.
- [ ] **Phase 2: Core Assistant and Session Continuity** - Stabilize the primary chat and persisted session experience.
- [ ] **Phase 3: Memory Durability and Write Policy** - Make memory trustworthy, durable, and inspectable.
- [ ] **Phase 4: Tool, MCP, and Coding Workflow Hardening** - Make tool execution and coding actions dependable for daily use.
- [ ] **Phase 5: Email and Voice Communications** - Ship production-credible email and phone or voice lanes.
- [ ] **Phase 6: Deployment, Runtime, and Operator Ops** - Make the runtime shippable, diagnosable, and recoverable.
- [ ] **Phase 7: Security, Observability, and Release Exit** - Close MVP trust gaps and verify release readiness.

### Phase Details

### Phase 1: Onboarding and First-Run Trust
**Goal**: A new operator can install OpenRustClaw, validate prerequisites, and reach a working first assistant session without rough edges.
**Depends on**: Nothing (first phase)
**Requirements**: [ONBD-01, ONBD-02, ONBD-03]
**Success Criteria** (what must be TRUE):
  1. Operator can follow the documented install and setup path without manual source edits.
  2. First-run validation catches missing env, provider, or channel prerequisites before runtime failure.
  3. A clean install reaches a working assistant session with clear operator feedback.
**Plans**: 2/3 plans executed

Plans:
- [x] 01-01: Harden doctor-backed first-start readiness gating for onboarding
- [x] 01-02: Add workflow-level onboarding regression coverage
- [ ] 01-03: Align installation and top-level first-run documentation

### Phase 2: Core Assistant and Session Continuity
**Goal**: The primary assistant conversation loop is reliable across restarts, reconnects, and normal operator use.
**Depends on**: Phase 1
**Requirements**: [ASST-01, ASST-02]
**Success Criteria** (what must be TRUE):
  1. User can start a primary assistant chat surface and exchange stable multi-turn conversations.
  2. Session state restores coherently after restart or reconnect.
  3. Primary assistant UX exposes continuity clearly enough that operators trust what was resumed.
**Plans**: TBD

Plans:
- [ ] TBD (run $gsd-plan-phase 2 to break down)

### Phase 3: Memory Durability and Write Policy
**Goal**: Memory becomes durable, policy-governed, and observable enough for production assistant use.
**Depends on**: Phase 2
**Requirements**: [MEM-01, MEM-02]
**Success Criteria** (what must be TRUE):
  1. Memory survives restart and reload in the supported production paths.
  2. Memory writes only occur under an explicit policy boundary instead of implicit opportunism.
  3. Operators can inspect what was stored, why it was stored, and what was recalled.
**Plans**: TBD

Plans:
- [ ] TBD (run $gsd-plan-phase 3 to break down)

### Phase 4: Tool, MCP, and Coding Workflow Hardening
**Goal**: Tool execution and coding workflows become dependable, bounded, and auditable for real operator work.
**Depends on**: Phase 3
**Requirements**: [TOOL-01, TOOL-02, CODE-01, CODE-02]
**Success Criteria** (what must be TRUE):
  1. Configured tools and MCP capabilities execute with clear permission and failure behavior.
  2. Coding workflows can inspect, edit, run, and verify code without leaving the workspace in a confusing state.
  3. Operators can review artifacts for tool and coding actions after execution.
**Plans**: TBD

Plans:
- [ ] TBD (run $gsd-plan-phase 4 to break down)

### Phase 5: Email and Voice Communications
**Goal**: Email and phone or voice assistant flows are reliable enough to be part of the MVP boundary.
**Depends on**: Phase 4
**Requirements**: [COMM-01, COMM-02]
**Success Criteria** (what must be TRUE):
  1. Email workflows can receive, send, and persist actionable assistant outcomes.
  2. Phone or voice flows can answer or participate in interactions and persist the result.
  3. Operators can diagnose failed communications from runtime artifacts.
**Plans**: TBD

Plans:
- [ ] TBD (run $gsd-plan-phase 5 to break down)

### Phase 6: Deployment, Runtime, and Operator Ops
**Goal**: Operators can deploy, run, observe, and recover OpenRustClaw through a production-ready operational path.
**Depends on**: Phase 5
**Requirements**: [OPS-01, OPS-02]
**Success Criteria** (what must be TRUE):
  1. Documented deployment and upgrade paths work for the supported runtime targets.
  2. Operators can start, stop, and recover the assistant runtime without ad hoc tribal knowledge.
  3. Health, logs, and diagnostics cover the main assistant subsystems.
**Plans**: TBD

Plans:
- [ ] TBD (run $gsd-plan-phase 6 to break down)

### Phase 7: Security, Observability, and Release Exit
**Goal**: MVP ships behind secure defaults, measurable runtime behavior, and an explicit release gate.
**Depends on**: Phase 6
**Requirements**: [SEC-01, REL-01]
**Success Criteria** (what must be TRUE):
  1. Production auth, secret handling, origin validation, and sandbox boundaries are secure by default.
  2. The MVP has end-to-end verification coverage across onboarding, assistant, memory, tools, coding, communications, and ops.
  3. Release readiness is captured in an operator-facing checklist with clear exit criteria.
**Plans**: TBD

Plans:
- [ ] TBD (run $gsd-plan-phase 7 to break down)

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Onboarding and First-Run Trust | 2/3 | In Progress|  |
| 2. Core Assistant and Session Continuity | 0/TBD | Not started | - |
| 3. Memory Durability and Write Policy | 0/TBD | Not started | - |
| 4. Tool, MCP, and Coding Workflow Hardening | 0/TBD | Not started | - |
| 5. Email and Voice Communications | 0/TBD | Not started | - |
| 6. Deployment, Runtime, and Operator Ops | 0/TBD | Not started | - |
| 7. Security, Observability, and Release Exit | 0/TBD | Not started | - |
