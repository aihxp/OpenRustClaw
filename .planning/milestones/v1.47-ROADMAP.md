# Roadmap: OpenRustClaw

## Milestones

- See `.planning/MILESTONES.md` for the full shipped milestone ledger.
- ✅ **v1.46 Ad Hoc Release Catch-Up and GSD Re-entry** — Phases 196-199 (shipped 2026-04-09)
- ✅ **v1.47 Runtime Lifecycle Reliability** — shipped 2026-04-09

## Current Status

`v1.47` shipped on 2026-04-09 and closed the runtime lifecycle reliability gap around listener conflicts, stale runtime state, and restart recovery.

- Next command: `$gsd-new-milestone`
- Queued after this milestone: release traceability and milestone correlation

## Current Milestone Overview

The goal of `v1.47` is to make `openrustclaw start`, `stop`, and `restart` trustworthy when the configured listener is already bound, the prior runtime is stale, or the runtime exited only partially. The milestone is driven by a real operator-facing failure where `restart` exited early and a follow-on `start` hit `127.0.0.1:18789` already in use.

## Phases

- [x] **Phase 200: Runtime Ownership and Conflict Classification** - Make lifecycle commands classify listener ownership and stale-runtime state before failing on a busy port.
- [x] **Phase 201: Restart and Stop Recovery Hardening** - Make stop and restart reconcile runtime locks, recoverable stale processes, and listener reuse safely.
- [x] **Phase 202: Operator Recovery Surface and Verification** - Ship the regression coverage and operator remediation surface for lifecycle conflict recovery.

## Phase Details

### Phase 200: Runtime Ownership and Conflict Classification
**Goal**: Make lifecycle commands classify listener ownership and stale-runtime state before failing on a busy port.
**Depends on**: Nothing (first phase)
**Requirements**: RUN-02, DIAG-01, DIAG-02
**Success Criteria** (what must be TRUE):
  1. `openrustclaw start` can tell the operator whether the configured listener is owned by a healthy OpenRustClaw runtime, stale OpenRustClaw state, or a foreign process before it exits.
  2. Listener-conflict output includes concrete ownership details or the strongest safe classification available for the blocked port.
  3. Common busy-port failures no longer collapse into a generic `address is already in use` error without remediation context.

### Phase 201: Restart and Stop Recovery Hardening
**Goal**: Make stop and restart reconcile runtime locks, recoverable stale processes, and listener reuse safely.
**Depends on**: Phase 200
**Requirements**: RUN-01, RUN-03
**Success Criteria** (what must be TRUE):
  1. `openrustclaw restart` can bring a recoverable workspace runtime back to a healthy listening state without requiring ad hoc manual cleanup.
  2. `openrustclaw stop` clears or reconciles stale runtime lock and listener metadata so the next `start` does not fail on OpenRustClaw-owned stale state.
  3. Lifecycle commands preserve clear boundaries between recoverable OpenRustClaw state and conflicts caused by foreign processes.

### Phase 202: Operator Recovery Surface and Verification
**Goal**: Ship the regression coverage and operator remediation surface for lifecycle conflict recovery.
**Depends on**: Phase 201
**Requirements**: DIAG-03, SAFE-01, SAFE-02
**Success Criteria** (what must be TRUE):
  1. Automated tests cover active-runtime conflicts, stale-lock recovery, foreign-process conflicts, and restart recovery.
  2. Operator-facing CLI output and docs provide one bounded remediation path for loopback listener conflicts and partial restart failures.
  3. The shipped runtime lifecycle documentation matches the actual behavior and limits of the implemented recovery flow.

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 200. Runtime Ownership and Conflict Classification | 1/1 | Complete | 2026-04-09 |
| 201. Restart and Stop Recovery Hardening | 1/1 | Complete | 2026-04-09 |
| 202. Operator Recovery Surface and Verification | 1/1 | Complete | 2026-04-09 |

## Recent Milestone

Archived detail lives in `.planning/milestones/v1.46-ROADMAP.md`.

<details>
<summary>v1.46 Ad Hoc Release Catch-Up and GSD Re-entry</summary>

- [x] Phase 196: Ad Hoc Release Audit and Catch-Up Ledger (1/1 plans, completed 2026-04-09)
- [x] Phase 197: Onboarding, Runtime, and Operator Journey Convergence (1/1 plans, completed 2026-04-09)
- [x] Phase 198: Release and Operational Hardening Capture (1/1 plans, completed 2026-04-09)
- [x] Phase 199: GSD Re-entry Closeout and Next Queue Definition (1/1 plans, completed 2026-04-09)

</details>

## Current Status

- Most recent milestone: `v1.47 Runtime Lifecycle Reliability`
- Roadmap progress: 3/3 phases complete
- Current work: milestone closeout and queue handoff to release traceability
- Next step: `$gsd-new-milestone`
