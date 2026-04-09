# Roadmap: OpenRustClaw

## Milestones

- ✅ **v1.0 through v1.45** - shipped. Full milestone history and archives: `.planning/MILESTONES.md`
- 🚧 **v1.46 Ad Hoc Release Catch-Up and GSD Re-entry** - active

## Overview

`v1.46` is a catch-up milestone for the work that was shipped directly on `main` after `v1.45` without passing through the GSD deck first. The goal is to make the planning state truthful again before new work continues inside GSD. This milestone treats the public `1.4.1` through `1.4.9` line as already shipped baseline, captures the major onboarding, runtime, release, and operator-hardening changes under one milestone story, and ends with a clean GSD re-entry point for future phases.

## Phases

**Phase Numbering:**
- Integer phases continue the live sequence from the prior milestone.
- This milestone starts at Phase 196 because `v1.45` ended at Phase 195.

- [ ] **Phase 196: Ad Hoc Release Audit and Catch-Up Ledger** - Inventory the shipped `1.4.1` through `1.4.9` changes and record them as one truthful milestone baseline.
- [ ] **Phase 197: Onboarding, Runtime, and Operator Journey Convergence** - Align the active planning story with the shipped onboarding, repair, delegated-agent, Tailscale, and runtime-control behavior.
- [ ] **Phase 198: Release and Operational Hardening Capture** - Preserve the shipped release-lane, update-check, OpenClaw migration, temp-dir mitigation, and runtime lifecycle hardening as canonical operational baseline.
- [ ] **Phase 199: GSD Re-entry Closeout and Next Queue Definition** - Finish the catch-up milestone with an explicit next phase so future work proceeds through GSD instead of ad hoc release flow.

## Phase Details

### Phase 196: Ad Hoc Release Audit and Catch-Up Ledger
**Goal**: Record the shipped `1.4.1` through `1.4.9` work as one truthful milestone baseline.
**Depends on**: Nothing (first phase)
**Requirements**: CAT-01, CAT-02
**Success Criteria** (what must be TRUE):
  1. The out-of-band semver releases are summarized once in canonical planning docs instead of only existing in commits and chat transcripts.
  2. The planning deck names the major shipped surfaces that changed across onboarding, doctor, runtime lifecycle, Tailscale, release handling, and migration.
  3. Future milestone work no longer depends on remembering which direct-to-main fixes already shipped.

### Phase 197: Onboarding, Runtime, and Operator Journey Convergence
**Goal**: The active planning story matches the shipped operator journey and runtime control baseline.
**Depends on**: Phase 196
**Requirements**: OPS-01, OPS-02
**Success Criteria** (what must be TRUE):
  1. Delegated local-agent onboarding, repair flow, Codex or Claude Code style access modes, and runtime control are represented truthfully in the live planning deck.
  2. Tailscale-aware gateway guidance and operator lifecycle controls are treated as shipped baseline, not pending ideas.
  3. The active milestone no longer describes a stale pre-hardening operator journey.

### Phase 198: Release and Operational Hardening Capture
**Goal**: Preserve the shipped release-lane and operational hardening work as canonical baseline.
**Depends on**: Phase 197
**Requirements**: OPS-03, REL-01, REL-02
**Success Criteria** (what must be TRUE):
  1. The planning deck reflects the shipped public semver line through `1.4.9`.
  2. Startup update checking, safe OpenClaw migration, repo-local temp-dir defaults, and runtime stop or restart are preserved as shipped operational improvements.
  3. Future work no longer needs to rediscover these operational fixes as undocumented repo lore.

### Phase 199: GSD Re-entry Closeout and Next Queue Definition
**Goal**: Finish the catch-up milestone with an explicit next queue inside GSD.
**Depends on**: Phase 198
**Requirements**: GSD-01
**Success Criteria** (what must be TRUE):
  1. The active planning deck gives one explicit next command and next phase for future work.
  2. The catch-up milestone closes with enough truthfulness that new work can stay inside GSD.
  3. The repo is ready for the next planning cycle without another round of ad hoc milestone reconstruction.

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 196. Ad Hoc Release Audit and Catch-Up Ledger | 0/0 | Not started | - |
| 197. Onboarding, Runtime, and Operator Journey Convergence | 0/0 | Not started | - |
| 198. Release and Operational Hardening Capture | 0/0 | Not started | - |
| 199. GSD Re-entry Closeout and Next Queue Definition | 0/0 | Not started | - |

## Current Status

- Active milestone: `v1.46 Ad Hoc Release Catch-Up and GSD Re-entry`
- Roadmap progress: 0/4 phases complete
- Current work: capture the shipped `1.4.1` through `1.4.9` work under one milestone and restore a truthful GSD starting point
- Next step: `$gsd-plan-phase 196`
