# Phase 165: Cleanup Inventory and Regression Baseline - Context

**Gathered:** 2026-03-29
**Status:** Completed

<domain>
## Phase Boundary

Build the cleanup inventory and the verification baseline that will guard all later deletion, merge, documentation, and workflow changes in `v1.40`.

</domain>

<decisions>
## Implementation Decisions

### public-surface first
Treat `README.md`, `docs/`, public Cargo package metadata, and GitHub workflow files as the first cleanup inventory because they are the product-facing surfaces the milestone is explicitly targeting.

### evidence before deletion
Do not remove or merge files until there is at least one concrete verification path that can show the cleanup did not regress the shipped product.

### release lane stays real
Use current registry state and current repo tags as part of the baseline so the release phases work from the actual public starting point rather than milestone memory.

</decisions>

<code_context>
## Existing Code Insights

- `cargo check --workspace` passes, but `openrustclaw-cli` currently emits dead-code warnings that will fail the `ci.yml` `cargo check --workspace` job because that workflow sets `RUSTFLAGS="-D warnings"`.
- `mdbook build docs` passes and regenerates `docs/book`.
- `cargo test -p openrustclaw-e2e-tests --test e2e_tests` passes at `89 passed, 0 failed, 3 ignored`.
- Public docs still expose internal migration vocabulary in `README.md`, `docs/src/architecture/greenfield-transition.md`, `docs/src/architecture/overview.md`, `docs/src/SUMMARY.md`, `docs/src/contributing/development.md`, `docs/feature-matrix.md`, and related mdBook mirrors.
- `crates/app/Cargo.toml` still uses internal migration language in its package description.
- `openrustclaw-core` is published on crates.io at `0.1.0`; `openrustclaw-cli` is not currently published.
- Git tags stop at `v1.15`, so public release tags are behind the milestone history recorded in planning docs.

</code_context>

<specifics>
## Specific Ideas

- Record public docs and metadata cleanup targets before editing them.
- Use compile, docs-build, and E2E checks as the minimum cleanup regression bundle.
- Treat workflow logic and warning debt as the likely first CI repair targets.

</specifics>

<deferred>
## Deferred Ideas

- Broad crate-publication expansion beyond the currently published `openrustclaw-core` contract unless later release prep proves a wider publish surface is ready

</deferred>
