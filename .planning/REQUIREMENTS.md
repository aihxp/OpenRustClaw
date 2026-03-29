# Requirements: v1.40 Public Product Cleanup, Documentation Convergence, CI Repair, and Release

**Started:** 2026-03-29
**Internal program baselines preserved:** seam ledger `18/18`, full-conversion roadmap `6/6`, native-delivery planning roadmap `8/8`, native-delivery implementation roadmap `6/6`, native-product E2E roadmap `1/1`
**Public-product convergence roadmap baseline before shipment:** `0/1`, or `0%`
**Public-product convergence roadmap target after shipment:** `1/1`, or `100%`

## Scope

This milestone cleans and simplifies the repo without breaking the shipped product, converges public docs and metadata to the product’s current public story, repairs failing GitHub Actions and release automation, and ships the next public release including crates.io publication.

## Milestone Requirements

- [ ] **PPC-01**: Public-facing docs, package metadata, and repo entrypoints no longer rely on internal migration terminology and instead describe the shipped product plainly.
- [ ] **PPC-02**: Stale or duplicated code, docs, workflows, and support files are deleted, merged, or simplified only where verification shows the cleanup is safe.
- [ ] **PPC-03**: Relevant GitHub Actions and release automation workflows are green, or intentionally retired with explicit rationale.
- [ ] **PPC-04**: The next public release is prepared and shipped with synchronized versions, release notes, package metadata, and crates.io publication state.

## Future Requirements

- The closed internal architecture and verification denominators must stay closed and should not be reused as public release or cleanliness percentages.
- Public docs should describe the product’s behavior, install story, release story, and supported entrypoints without surfacing internal migration history as the primary narrative.
- Cleanup work should prefer deletion and convergence over adding more parallel files, modules, or workflow variants.
- Release work after `v1.40` should build on the repaired automation and synchronized docs rather than reopening basic package-surface truthfulness.

## Out of Scope

- Large net-new product capabilities unrelated to cleanup, docs convergence, CI repair, or release
- Reopening the closed internal migration and verification programs as active execution queues
- Cosmetic rewrites that do not improve product truthfulness, repo clarity, or release readiness
- Unsafe cleanup that cannot be verified against the shipped product behavior

## Traceability

- `PPC-01` -> Phase 166
- `PPC-02` -> Phases 165 and 167
- `PPC-03` -> Phase 168
- `PPC-04` -> Phases 169 and 170
