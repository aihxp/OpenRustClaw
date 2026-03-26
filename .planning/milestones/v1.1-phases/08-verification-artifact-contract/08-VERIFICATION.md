---
phase: "08"
verified: 2026-03-26T17:38:30.000Z
status: passed
score: "3/3 must-haves verified"
---

# Phase 8: verification-artifact-contract — Verification

## Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `init milestone-op` reports the active roadmap truth for v1.1, including `phase_count: 3` and archived milestone inventory. | passed | `node .codex/get-shit-done/bin/gsd-tools.cjs init milestone-op --raw` |
| 2 | `phase complete` now fails fast when verification is missing or stale and succeeds only with a current passing verification artifact. | passed | Temp workspace probes: missing verification returns `No VERIFICATION.md artifact exists for this phase`; stale verification returns `08-VERIFICATION.md predates 08-01-SUMMARY.md`; current passing verification completes successfully |
| 3 | Operators can inspect verification readiness debt and manual scaffolds now emit schema-compatible verification artifacts. | passed | `node .codex/get-shit-done/bin/gsd-tools.cjs audit-uat --raw`; temp workspace `scaffold verification --phase 8 --raw` produced `verified`, `status`, and `score` frontmatter |

## Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `.codex/get-shit-done/bin/lib/verification-artifacts.cjs` | Shared verification-readiness inspection helpers | passed | Added in commit `8e8c965` |
| `.codex/get-shit-done/bin/lib/init.cjs` | Roadmap-truthful milestone bootstrap counts and archive detection | passed | Updated in commit `8e8c965` |
| `.codex/get-shit-done/bin/lib/phase.cjs` | Hard phase-complete gate on missing, stale, pending, or failed verification | passed | Updated in commit `8e8c965` |
| `.codex/get-shit-done/bin/lib/uat.cjs` | Synthetic verification debt reporting for missing and stale artifacts | passed | Updated in commit `185d150` |
| `.codex/get-shit-done/bin/lib/commands.cjs` | Direct verification scaffold aligned with the active schema | passed | Updated in commit `c0ee1a4` |
| `.codex/get-shit-done/workflows/help.md` | Help text that describes verification readiness debt | passed | Updated in commit `185d150` |
| `.codex/get-shit-done/workflows/progress.md` | Progress docs that show verification debt categories | passed | Updated in commit `185d150` |
| `.codex/get-shit-done/workflows/execute-phase.md` | Execute-phase guidance that states the hard verification gate | passed | Updated in commit `c0ee1a4` |

## Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `init milestone-op` | `.planning/ROADMAP.md` current milestone | `extractCurrentMilestone()` + phase heading scan | passed | v1.1 bootstrap now counts roadmap phases before phase directories are complete |
| `phase complete` | `VERIFICATION.md` truth | `inspectVerificationArtifacts()` | passed | One shared readiness check now gates completion instead of ad-hoc warnings |
| `audit-uat` | Verification lifecycle debt | `inspectVerificationArtifacts()` + synthetic debt items | passed | Missing or stale verification now appears in operator audit output even without parseable human-verification rows |
| `scaffold verification` | Verification schema | structured frontmatter/body output | passed | Direct scaffolded files now match the declared `phase/verified/status/score` contract |

## Requirements Coverage

| Requirement | Status | Blocking Issue |
|-------------|--------|----------------|
| LIFE-01 | passed | |
| LIFE-02 | passed | |

## Result

Phase 8 passes. The lifecycle contract now requires a real, current `VERIFICATION.md` before phase completion, surfaces missing or stale verification debt to operators, and keeps direct verification scaffolding aligned with the schema the gates enforce.
