---
phase: 08-verification-artifact-contract
plan: 01
subsystem: verification-lifecycle-core
tags:
  - verification
  - lifecycle
  - planning
  - cli
provides:
  - Shared verification-readiness inspection for phase directories
  - Truthful milestone bootstrap counts from the active roadmap
  - Hard phase-complete gate on missing or stale verification artifacts
affects:
  - GSD milestone bootstrap
  - GSD phase completion
  - Verification artifact lifecycle contract
tech-stack:
  added: []
  patterns:
    - Reuse one shared verification inspection result across lifecycle commands instead of duplicating status checks
key-files:
  created:
    - .codex/get-shit-done/bin/lib/verification-artifacts.cjs
  modified:
    - .codex/get-shit-done/bin/lib/init.cjs
    - .codex/get-shit-done/bin/lib/phase.cjs
key-decisions:
  - Roadmap phases, not existing phase directories, define milestone phase counts
  - Phase completion now fails fast when verification is missing, stale, pending, or reports gaps
patterns-established:
  - Verification freshness is judged against the newest summary or UAT evidence in the phase directory
duration: 35min
completed: 2026-03-26
---

# Phase 8: Verification Artifact Contract Summary

**Hardened the core lifecycle contract so milestone bootstrap and phase completion now respect real verification state instead of optimistic filesystem assumptions.**

## Performance
- **Duration:** ~35 min
- **Tasks:** 3 completed
- **Files modified:** 3

## Accomplishments
- Added a shared verification inspector that finds the phase verification artifact, checks required frontmatter, and flags stale verification against newer summary or UAT evidence.
- Fixed `init milestone-op` so a brand-new milestone reports roadmap-truthful phase counts and archived milestone inventory even before phase directories are fully populated.
- Changed `phase complete` to fail before mutating roadmap or state when verification is missing, pending, stale, or already reports gaps.

## Verification
- `node .codex/get-shit-done/bin/gsd-tools.cjs init milestone-op --raw`
- Temp workspace probe: `phase complete 8 --raw` fails with no `VERIFICATION.md`
- Temp workspace probe: `phase complete 8 --raw` fails when `08-VERIFICATION.md` predates `08-01-SUMMARY.md`
- Temp workspace probe: `phase complete 8 --raw` succeeds when `08-VERIFICATION.md` is current and `status: passed`

## Next Phase Readiness
Plan 02 can now surface the same verification contract through cross-phase audit output instead of reconstructing debt heuristically.
