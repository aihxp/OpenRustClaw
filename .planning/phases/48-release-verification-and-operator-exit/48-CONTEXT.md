# Phase 48: Release Verification and Operator Exit - Context

**Gathered:** 2026-03-28
**Status:** Ready for execution

## Goal

Leave one repeatable operator runbook and verification bundle for future tagged releases.

## What We Know

- The repo already has a release checklist and release gate, but not a release-binaries workflow health check.
- `scripts/github-actions-admin.sh` now has a dedicated `check-release-binaries` command.
- The milestone should only close once the live tag run proves the end-to-end release path.

## Constraints

- The operator flow should stay in the existing admin and release docs, not scatter into another disconnected guide.
- Milestone closeout should preserve the exact live workflow evidence used to declare the release path healthy.

## Implementation Direction

- document the manual branch dry-run plus tag validation flow
- use the admin helper as the canonical live check
- archive the successful `v1.10` release workflow evidence during milestone closeout
