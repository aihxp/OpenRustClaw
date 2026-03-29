# Markdown Canonical Surfaces

**Created:** 2026-03-29
**Purpose:** Define which Markdown surfaces are canonical, mirrored, archive-only, or internal for the `v1.41` cleanup milestone.

## Canonical Public Surfaces

| Surface class | Canonical files | Why |
| --- | --- | --- |
| Repo entry | `README.md`, `SECURITY.md` | Primary GitHub-facing product and security entry surfaces |
| Planning-facing public docs | `docs/docs-audit.md`, `docs/documentation-contract.md`, `docs/feature-matrix.md`, `docs/product-positioning.md`, `docs/roadmap.md`, `docs/surface-matrix.md` | Root planning docs are the authoritative public planning-facing record |
| Guided product docs | `docs/src/**` outside `docs/src/planning/` | mdBook source for getting started, guides, deployment, operations, architecture, and API reference |
| Package docs | `crates/*/README.md` | Canonical package-facing documentation for crates.io and source browsing |
| Compatibility subproject docs | `sidecar/README.md`, `tests/e2e/README.md` | Canonical documentation for those scoped subtrees |

## Canonical Internal Surfaces

| Surface class | Canonical files | Why |
| --- | --- | --- |
| Live planning state | `.planning/PROJECT.md`, `.planning/REQUIREMENTS.md`, `.planning/ROADMAP.md`, `.planning/STATE.md` | Current milestone truth |
| Live codebase maps | `.planning/codebase/*.md` | Maintained planning references when still active |
| Internal coding guidance | `CLAUDE.md` | Active repo-local helper guidance for AI-assisted editing |

## Mirror Rules

- `docs/src/planning/*.md` is a mirror layer for the root `docs/*.md` planning-facing docs.
- Mirrors should stay thin and navigation-oriented.
- Mirrors should not become a second full-content copy of the same planning page.

## Archive-Only Surfaces

- `.planning/milestones/**`
- `.planning/milestones/*-phases/**`
- milestone verification bundles and archived requirements or roadmaps

These files are historical records. They can be referenced, but they are not candidates for style cleanup unless a file is materially broken.

## Internal System Surfaces

- `.codex/**`

These files are part of the workflow system, not the public repo documentation story.

## v1.41 Canonical Decisions

1. Keep root `docs/*.md` planning pages as canonical.
2. Keep `docs/src/planning/*.md` as thin mdBook mirrors only.
3. Delete orphaned one-off Markdown snapshots when a stronger canonical page already owns the topic.
4. Refresh stale internal helper docs in place instead of letting downstream copied snippets drift.
5. Keep crate README files as canonical package docs, but normalize wording when it conflicts with the current public product language.
