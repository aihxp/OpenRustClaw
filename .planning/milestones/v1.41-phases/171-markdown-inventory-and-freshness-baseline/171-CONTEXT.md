# Phase 171: Markdown Inventory and Freshness Baseline - Context

**Gathered:** 2026-03-29
**Status:** Complete

## Phase Boundary

Inventory the tracked Markdown surface, separate active repo docs from internal archives and workflow docs, and identify the stale or overlapping files that actually need cleanup.

## Key Findings

- The repo tracks `1285` Markdown files in Git.
- `995` live under `.planning/` and `188` under `.codex/`, so only `102` tracked Markdown files are part of the active repo/product/package surface.
- The highest-signal cleanup targets are not broad rewrites; they are a small set of stale or overlapping files:
  - `LLM_SDK_SUMMARY.md`
  - `docs/src/planning/roadmap.md`
  - `CLAUDE.md`
  - duplicated stale `CLAUDE.md` sample in `docs/src/guides/cursor-integration.md`

## Output Targets

- `.planning/codebase/MARKDOWN-SURFACE-INVENTORY.md`
- phase summary and verification bundle
