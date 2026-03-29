# Phase 166: Public Documentation and Metadata Convergence - Context

**Gathered:** 2026-03-29
**Status:** Completed

<domain>
## Phase Boundary

Remove internal migration terminology from public surfaces and make the public docs, package metadata, and release-facing entrypoints tell one consistent product story.

</domain>

<decisions>
## Implementation Decisions

### public copy only
Treat `README.md`, `docs/`, public Cargo metadata, and public-facing examples as the convergence boundary. Internal planning history can keep milestone language.

### truthful product language
Replace internal transition vocabulary with plain product terms such as application services, operator surfaces, compatibility surfaces, and public release versions.

### one public release story
The package and public docs should describe the shipped product and its release lane, not old milestone memory or internal migration staging.

</decisions>

<code_context>
## Existing Code Insights

- Public docs and package metadata exposed internal migration terms in architecture docs, contributor docs, and `crates/app/Cargo.toml`.
- The architecture page was better replaced than edited in place because the old file name itself exposed internal language.
- `README.md` was already product-facing and mainly needed consistency against docs rather than a wholesale rewrite.

</code_context>

<specifics>
## Specific Ideas

- Replace `greenfield-transition.md` with an application-boundaries page.
- Update mdBook navigation and related docs to point at the new public terminology.
- Keep internal planning docs intact while removing the language from public docs and package metadata.

</specifics>

<deferred>
## Deferred Ideas

- Renaming internal code modules that still carry historical names, unless that rename materially improves the public product surface in a later queue

</deferred>
