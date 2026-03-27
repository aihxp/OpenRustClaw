# Phase 38: Repo Hygiene and Drift Reduction - Context

**Gathered:** 2026-03-27
**Status:** Ready for planning and execution
**Mode:** Autonomous cleanup execution

<domain>
## Phase Boundary

This phase removes the most obvious brownfield drift that is already documented: CI still points at deleted parity-doc filenames, and the sidecar local Python state needs to be treated explicitly as non-canonical repo noise.

</domain>

<decisions>
## Implementation Decisions

### fix canonical doc references instead of restoring deleted names
The docs contract already says the canonical planning-facing files are `feature-matrix.md`, `surface-matrix.md`, and `product-positioning.md`. CI should be updated to those files instead of recreating dead parity filenames.

### protect local sidecar state without deleting user environments
This phase should tighten ignore and documentation boundaries around local sidecar artifacts, not destroy a developer's local `.venv`.

</decisions>

<code_context>
## Existing Code Insights

- `.github/workflows/ci.yml` still checks `docs/parity-matrix.md` and `docs/parity-positioning.md`, which no longer exist.
- `.gitignore` already ignores generic Python artifacts, but sidecar-specific local state is worth documenting explicitly because it sits inside the repo tree.
- `sidecar/README.md` is the right place to state that local Python state is non-canonical.

</code_context>

<specifics>
## Specific Ideas

- align CI to the current canonical docs and headings
- make sidecar local-artifact boundaries explicit in `.gitignore` and `sidecar/README.md`

</specifics>

<deferred>
## Deferred Ideas

- a stronger hygiene script can land in Phase 40 with the cleanup verification bundle

</deferred>
