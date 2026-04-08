# Roadmap: OpenRustClaw

## Milestones

- ✅ **v1.0 through v1.44** - shipped. Full milestone history and archives: `.planning/MILESTONES.md`
- 🚧 **v1.45 Agent Fabric, Routing Console, and Guided Delegation** - in progress

## Overview

`v1.45` builds on the truthful delegated-agent baseline from `v1.44` instead of reopening it. This milestone turns delegated local agents into a richer execution fabric by validating Cursor honestly, adding trusted remote backends, exposing a real routing console, and making the first-task path after onboarding feel intentional instead of generic.

## Phases

**Phase Numbering:**
- Integer phases continue the live sequence from the prior milestone.
- This milestone starts at Phase 191 because `v1.44` ended at Phase 190.

- [ ] **Phase 191: Cursor Surface Verification and Backend Expansion** - Confirm Cursor’s programmable surface truthfully and either add a supported backend lane or preserve detection-only behavior with explicit reasons.
- [ ] **Phase 192: Trusted Remote Backend Registry and Fabric Signals** - Add trusted remote hosts and routeable delegated backend inventory beyond one machine.
- [ ] **Phase 193: Delegated Route Policy, Audit, and Recovery** - Extend delegated execution controls so multi-host routes stay bounded, attributable, and recoverable.
- [ ] **Phase 194: Agent Routing Console and Operator Policy UX** - Build a dedicated routing console across Control UI, CLI, and inspect surfaces.
- [ ] **Phase 195: Guided First-Task Orchestration and Fallbacks** - Make the first real task after onboarding or repair choose a coherent route with actionable fallback guidance.

## Phase Details

### Phase 191: Cursor Surface Verification and Backend Expansion
**Goal**: OpenRustClaw can describe Cursor truthfully and, if supported, execute through it without weakening the current delegated-backend contract.
**Depends on**: Nothing (first phase)
**Requirements**: CURS-01, CURS-02, CURS-03
**Success Criteria** (what must be TRUE):
  1. Cursor support is explicitly classified as executable or detection-only using documented evidence instead of guesswork.
  2. If Cursor is execution-capable, it routes through the same bounded audit and policy path as existing delegated backends.
  3. If Cursor is not execution-capable, every operator surface says so clearly and consistently.
**Plans**: TBD

### Phase 192: Trusted Remote Backend Registry and Fabric Signals
**Goal**: Delegated routing can reason about trusted remote agent backends instead of only local CLIs on one machine.
**Depends on**: Phase 191
**Requirements**: FABR-01, FABR-02
**Success Criteria** (what must be TRUE):
  1. Operators can register trusted remote hosts and inspect their delegated backend inventory.
  2. Route selection can compare local and remote delegated backends using readiness, policy, and compatibility signals.
  3. Remote backend inventory remains explicit and operator-enrolled instead of ambient or hidden.
**Plans**: TBD

### Phase 193: Delegated Route Policy, Audit, and Recovery
**Goal**: Multi-host delegated execution stays bounded and recoverable under one policy and audit contract.
**Depends on**: Phase 192
**Requirements**: FABR-03, FABR-04
**Success Criteria** (what must be TRUE):
  1. Remote delegated execution preserves audit evidence, operator attribution, and runtime constraints.
  2. Route decisions record why a backend was chosen, blocked, or failed.
  3. Operators get actionable recovery hints and fallback route cues when delegated execution cannot proceed.
**Plans**: TBD

### Phase 194: Agent Routing Console and Operator Policy UX
**Goal**: Operators have one dedicated routing console for backend inventory, policy, readiness, and receipts instead of stitching that story together manually.
**Depends on**: Phase 193
**Requirements**: ROUTX-01, ROUTX-02, ROUTX-03
**Success Criteria** (what must be TRUE):
  1. Control UI exposes a dedicated routing console with backend inventory, routeable capacity, and policy state.
  2. CLI and Control UI can manage delegated backend policy without raw file editing.
  3. Route decisions and delegated receipts stay legible across Control UI, inspect, and CLI surfaces.
**Plans**: TBD

### Phase 195: Guided First-Task Orchestration and Fallbacks
**Goal**: The first task after onboarding or repair uses the selected lane and available backend fabric to guide operators into a real execution path instead of generic defaults.
**Depends on**: Phase 194
**Requirements**: TASK-01, TASK-02, TASK-03
**Success Criteria** (what must be TRUE):
  1. First-task suggestions reflect the selected lane, available backends, and current policy state.
  2. Orchestration can prefill or recommend a coherent initial route, claw, and profile path.
  3. Blocked or unsupported preferred lanes produce actionable fallback choices before execution starts.
**Plans**: TBD

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 191. Cursor Surface Verification and Backend Expansion | 0/2 | Not Started | — |
| 192. Trusted Remote Backend Registry and Fabric Signals | 0/2 | Not Started | — |
| 193. Delegated Route Policy, Audit, and Recovery | 0/2 | Not Started | — |
| 194. Agent Routing Console and Operator Policy UX | 0/2 | Not Started | — |
| 195. Guided First-Task Orchestration and Fallbacks | 0/2 | Not Started | — |

## Current Status

- Active milestone: v1.45 Agent Fabric, Routing Console, and Guided Delegation
- Roadmap progress: 0/5 phases complete
- Current work: milestone initialized; Phase 191 is next
- Next step: `$gsd-plan-phase 191`
