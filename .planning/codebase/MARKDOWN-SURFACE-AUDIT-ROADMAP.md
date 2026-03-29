# Markdown Surface Audit Roadmap

**Created:** 2026-03-29
**Purpose:** Canonical follow-on roadmap for auditing every non-generated Markdown surface in the repository, then updating, merging, or deleting stale documentation cleanly.
**Status:** Active at `0/1` shipped milestones, or `0%`
**Baselines preserved:** internal architecture roadmap families closed; public-product convergence roadmap closed at `1/1`

## What This Queue Means

The previous public-product convergence queue cleaned the outward-facing product story and release line. This queue is narrower and deeper: audit the entire Markdown surface that still matters, decide which files are canonical, and remove drift by merging or deleting the rest.

This roadmap measures:

- a complete inventory of non-generated Markdown files
- explicit canonical ownership for overlapping documentation surfaces
- bounded merge or deletion of stale or duplicated Markdown
- post-cleanup verification that the remaining Markdown surface is internally consistent

## Milestone Sequence

### v1.41 Markdown Surface Audit, Cleanup, and Consolidation

Primary target: produce a repo-wide Markdown inventory, clean stale or overlapping docs, and leave one synchronized canonical documentation surface behind.

- inventory and freshness baseline for all non-generated Markdown
- canonical surface definition and overlap map
- cleanup pass across repo-root docs, docs-site pages, crate READMEs, and live planning references
- final verification and deletion rationale archive

## Exit Criteria

OpenRustClaw should only claim this queue complete when all of the following are true:

- every non-generated Markdown file in scope has a documented disposition
- stale or duplicated Markdown has either been merged into a canonical surface or deleted with rationale
- canonical Markdown surfaces agree on public product versioning and current product language
- the final verification bundle records the kept, merged, deleted, and archive-only Markdown surfaces truthfully

## Companion Documents

- `.planning/PROJECT.md` — project-level milestone context and decisions
- `.planning/REQUIREMENTS.md` — active milestone scope and acceptance criteria
- `.planning/ROADMAP.md` — live phase sequence for the active milestone
