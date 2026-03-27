# Phase 32: Documentation Inventory and Canonical Source Contract - Context

**Gathered:** 2026-03-27
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Phase 32 defines the documentation ownership model for the repo before the bigger rewrite begins. The goal is to stop treating `README.md`, repo-root planning docs under `docs/`, and the mdBook under `docs/src/` as equally authoritative when they overlap.

</domain>

<decisions>
## Implementation Decisions

### Canonical ownership comes first
- Treat documentation structure as product infrastructure for this milestone.
- Make each major docs family explicit: repo entrypoint, guided docs, planning mirrors, security policy, and milestone planning.
- Prefer one canonical source plus mirrors or stubs over parallel full-content copies.

### Root planning docs remain canonical for shipped-surface matrices
- Keep `docs/feature-matrix.md`, `docs/surface-matrix.md`, `docs/roadmap.md`, and `docs/product-positioning.md` as the canonical planning-facing product docs.
- Keep `docs/src/planning/*.md` as mdBook entry points that point back to the canonical root files.

### Document merge and delete rules before rewriting content
- Add an explicit documentation contract that names canonical, mirrored, and removable surfaces.
- Use that contract to justify later rewrite, merge, and delete work during the milestone.

### the agent's Discretion
The exact cleanup list and audience framing can be refined during later phases as long as they preserve the canonical-source contract established here.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `docs/feature-matrix.md`, `docs/surface-matrix.md`, `docs/roadmap.md`, and `docs/product-positioning.md` already act as canonical root planning docs.
- `docs/src/planning/*.md` already follows a stub-or-mirror pattern and can be formalized instead of reinvented.
- `docs/docs-audit.md` already exists as a root audit artifact and can later absorb maintenance expectations.

### Established Patterns
- The repo already uses repo-root Markdown for canonical planning artifacts and mdBook pages for navigation.
- Recent milestones prefer truthful docs over aspirational language.

### Integration Points
- `README.md` is the repo entrypoint and must reference the canonical docs map.
- `docs/src/SUMMARY.md` controls mdBook navigation and should expose the documentation contract.
- Phase progress later needs to stay synced in `.planning/ROADMAP.md` and `.planning/STATE.md`.

</code_context>

<specifics>
## Specific Ideas

- add a `docs/documentation-contract.md` file that explains ownership and cleanup rules
- add a mdBook planning stub for that contract
- use the contract to guide the README rewrite and getting-started convergence that follow

</specifics>

<deferred>
## Deferred Ideas

- exact wording and information architecture for the public-facing README and docs introduction
- detailed merge or delete actions for obsolete docs pages

</deferred>
