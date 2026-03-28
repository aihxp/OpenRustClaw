# Phase 43: GitHub Actions Audit and Repair - Context

**Gathered:** 2026-03-27
**Status:** Ready for execution

## Goal

Restore confidence in the public automation surface by making workflows, badges, and release jobs reflect the current verification contract.

## What We Know

- Live GitHub Actions runs on `main` show repeated `Shipped Surface CI` failures.
- The failing job is `Parity Inventory Contract`, specifically `Validate parity contract docs`.
- The failure log shows `rg: command not found` on the GitHub runner before any package install step.

## Constraints

- The CI fix should not depend on undocumented runner packages.
- Repo hygiene checks should still work locally even when `rg` is unavailable.

## Implementation Direction

- install `ripgrep` explicitly in the parity-inventory job
- make `scripts/check-repo-hygiene.sh` fall back to `grep` when `rg` is unavailable
- verify the parity bundle locally before pushing
