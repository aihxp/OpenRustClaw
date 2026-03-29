# Requirements: v1.41 Markdown Surface Audit, Cleanup, and Consolidation

**Started:** 2026-03-29
**Status:** Active
**Roadmap companion:** `.planning/codebase/MARKDOWN-SURFACE-AUDIT-ROADMAP.md`

## Goal

Audit every non-generated Markdown surface in the repository, classify which files are canonical versus stale or duplicated, and then update, merge, or delete those files cleanly so the remaining Markdown surface is truthful, synchronized, and maintainable.

## Active Requirements

- [ ] **MDA-01 Markdown Inventory:** Produce a repo-wide inventory of non-generated Markdown files with ownership, audience, freshness, overlap, and proposed disposition.
- [ ] **MDA-02 Canonical Surface:** Define which Markdown files are canonical for repo entry, product docs, crate READMEs, planning history, and archive-only material.
- [ ] **MDA-03 Cleanup Execution:** Apply bounded rewrites, merges, moves, or deletions to stale or duplicated Markdown files without breaking the shipped documentation story.
- [ ] **MDA-04 Consistency Verification:** Revalidate the remaining Markdown surface for internal links, version language, duplicated claims, and cross-surface consistency after cleanup.

## In Scope

- Repository-root Markdown files such as `README.md`, adjacent product docs, and contributor docs.
- `docs/` source Markdown, excluding generated `docs/book/`.
- Crate-level `README.md` files that affect public package surfaces.
- Planning Markdown that still acts as a live or referenced contract instead of historical archive only.
- Markdown deletion or merge decisions when they can be justified by a stronger canonical replacement.

## Out of Scope

- Generated docs output such as `docs/book/`.
- Non-Markdown code, runtime, or workflow changes except where required to keep docs references truthful.
- Rewriting historical shipped milestone archives for style alone when they are not part of the active public surface.
- Marketing-site redesign work detached from the repo’s canonical documentation surfaces.

## Acceptance Criteria

- The repo has a documented Markdown inventory and disposition map.
- Duplicate or stale Markdown files are merged or removed where justified.
- Remaining canonical Markdown surfaces agree on current product versioning and product language.
- The final verification bundle records what changed, what was deleted, and why the remaining surfaces are the canonical ones.
