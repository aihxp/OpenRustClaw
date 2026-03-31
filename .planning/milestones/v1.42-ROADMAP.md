# Roadmap: OpenRustClaw

## Milestones

- ✅ **v1.0 through v1.41** - shipped. Full milestone history and archives: `.planning/MILESTONES.md`
- 🚧 **v1.42 Onboarding Primary LLM Selection** - Phases 177-180

## Overview

`v1.42` keeps the milestone bounded to CLI onboarding, setup-state and handoff continuity, related operator verification surfaces, and the docs or regression work needed to keep that path truthful. It does not broaden into a generic provider framework or a Control UI redesign.

## Phases

**Phase Numbering:**
- Integer phases continue the live sequence from the prior milestone.
- This milestone starts at Phase 177 because `v1.41` ended at Phase 176.

- [x] **Phase 177: Provider Access Modes and Resume State** - Choose a supported onboarding provider path and persist access-mode context without losing progress. (completed 2026-03-30)
- [x] **Phase 178: Live Verification and Bootstrap Evidence** - Verify the chosen provider lane before readiness and classify failures truthfully. (completed 2026-03-30)
- [x] **Phase 179: Primary Model Discovery and Persistence** - Select an explicit primary task model and persist provider plus model together in onboarding. (completed 2026-03-30)
- [x] **Phase 180: Handoff, Repair, and First-Launch Continuity** - Surface the selected lane in setup handoff and reuse it on resume, repair, and first launch. (completed 2026-03-30)

## Phase Details

### Phase 177: Provider Access Modes and Resume State
**Goal**: Operators can choose a truthful onboarding provider path and resume the model setup step without losing valid provider or access-mode context.
**Depends on**: Nothing (first phase)
**Requirements**: ACCS-01, ACCS-02, ACCS-03, ACCS-04
**Success Criteria** (what must be TRUE):
  1. Operator can choose the primary LLM provider from the onboarding-supported provider list during the model setup step.
  2. Operator sees only the access modes that actually apply to the selected provider, including API key, subscription-managed, local-runtime, or bounded combinations where applicable.
  3. Onboarding requests only the credential or runtime input required for the chosen provider path instead of asking for irrelevant secrets.
  4. The model setup step can resume without losing the previously selected provider and access mode when those choices are still valid.
**Plans**: 1/1 plans complete

Plans:
- [x] 177-01 Add provider-path selection, resume persistence, and setup-state propagation

### Phase 178: Live Verification and Bootstrap Evidence
**Goal**: Onboarding can prove the selected provider lane is actually usable before the model step claims readiness.
**Depends on**: Phase 177
**Requirements**: VERF-01, VERF-02, VERF-03
**Success Criteria** (what must be TRUE):
  1. Onboarding verifies the chosen provider connection before the model setup step is marked ready.
  2. Operator can see whether verification failed because of authentication or access, missing local runtime, model unavailability, billing or quota, rate limiting, or generic provider reachability issues.
  3. Verification outcomes are recorded in durable bootstrap evidence so repair and resume can target the failed verification sub-step directly.
**Plans**: 1/1 plans complete

Plans:
- [x] 178-01 Enrich provider verification evidence, classification, and recovery routing

### Phase 179: Primary Model Discovery and Persistence
**Goal**: Operators can choose an explicit primary task model during onboarding and have that provider-model pair persisted through the shared runtime mutation lane.
**Depends on**: Phase 178
**Requirements**: MODL-01, MODL-02, MODL-03, MODL-04, MODL-05
**Success Criteria** (what must be TRUE):
  1. Onboarding can discover or scan available models for the chosen provider when that provider and access mode expose a usable live catalog.
  2. Operator can choose the primary task model explicitly from the discovered model list when discovery succeeds.
  3. Operator can enter a primary task model manually when live model discovery is unavailable, account-scoped, empty, or unsupported for the selected provider path.
  4. The selected provider and primary task model are persisted into runtime configuration in the same onboarding flow without requiring a separate post-setup runtime switch step.
  5. Onboarding records whether the selected primary model came from live discovery, recommended fallback, or manual entry.
**Plans**: 1/1 plans complete

Plans:
- [x] 179-01 Add live model discovery, explicit selection, and provider-model persistence

### Phase 180: Handoff, Repair, and First-Launch Continuity
**Goal**: Setup handoff and first launch remain aligned with the provider, access mode, and primary model established during onboarding.
**Depends on**: Phase 179
**Requirements**: HNDF-01, HNDF-02, HNDF-03
**Success Criteria** (what must be TRUE):
  1. Setup handoff surfaces show the selected provider, access mode, primary task model, and current readiness outcome for the onboarding model lane.
  2. Resume and repair flows can distinguish incomplete provider selection, failed verification, and incomplete model selection when re-entering the onboarding model step.
  3. First assistant launch after onboarding uses the provider and primary task model established during onboarding when the workspace is otherwise ready.
**Plans**: 1/1 plans complete

Plans:
- [x] 180-01 Align handoff detail, repair guidance, and first launch with the selected onboarding model lane

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 177. Provider Access Modes and Resume State | 1/1 | Complete | 2026-03-30 |
| 178. Live Verification and Bootstrap Evidence | 1/1 | Complete | 2026-03-30 |
| 179. Primary Model Discovery and Persistence | 1/1 | Complete | 2026-03-30 |
| 180. Handoff, Repair, and First-Launch Continuity | 1/1 | Complete | 2026-03-30 |

## Current Status

- Active milestone: v1.42 Onboarding Primary LLM Selection
- Roadmap progress: 4/4 phases complete
- Next step: `$gsd-audit-milestone`
