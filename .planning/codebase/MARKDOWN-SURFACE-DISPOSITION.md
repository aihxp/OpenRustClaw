# Markdown Surface Disposition Report

**Created:** 2026-03-29
**Purpose:** Final keep/merge/delete report for `v1.41 Markdown Surface Audit, Cleanup, and Consolidation`.

## Deleted

- `LLM_SDK_SUMMARY.md`
  - Reason: orphaned one-off implementation snapshot with no inbound references
  - Replacement: provider coverage context absorbed into `docs/src/architecture/provider-sdks.md`

## Normalized In Place

- `docs/src/planning/roadmap.md`
  - Changed from a drifting pseudo-roadmap into a thin mirror of the canonical root roadmap
- `CLAUDE.md`
  - Refreshed stale repo facts and outdated scheduling guidance
- `docs/src/guides/cursor-integration.md`
  - Updated the embedded `CLAUDE.md` sample to match the refreshed internal guidance
- `docs/src/guides/providers.md`
  - Clarified that the page covers common provider setups rather than the full provider inventory

## Kept As Canonical

- `README.md`
- `SECURITY.md`
- `docs/*.md` planning-facing pages
- `docs/src/**` product docs and API reference pages
- `crates/*/README.md` package-facing documentation
- `sidecar/README.md`
- `tests/e2e/README.md`

## Kept As Mirrors

- `docs/src/planning/docs-audit.md`
- `docs/src/planning/documentation-contract.md`
- `docs/src/planning/feature-matrix.md`
- `docs/src/planning/product-positioning.md`
- `docs/src/planning/roadmap.md`
- `docs/src/planning/surface-matrix.md`

## Kept As Historical Or Internal

- `.planning/milestones/**` and archived phase artifacts
- `.codex/**`
- `slides/**`

These surfaces remain tracked, but they are not part of the public canonical documentation story that `v1.41` cleaned.
