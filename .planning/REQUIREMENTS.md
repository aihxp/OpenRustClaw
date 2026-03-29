# Requirements: v1.38 Native Delivery Implementation: Native Product Verification and Packaging Exit

**Started:** 2026-03-28
**Historical greenfield baseline:** retired `18/18` ranked seam ledger complete, or `100%`
**Full-conversion roadmap baseline:** `6/6` milestones shipped, or `100%`
**Native-delivery planning roadmap baseline:** `8/8` milestones shipped, or `100%`
**Native-delivery implementation roadmap baseline before shipment:** `5/6` milestones shipped, or about `83%`
**Native-delivery implementation roadmap target after shipment:** `6/6` milestones shipped, or `100%`

## Scope

This milestone continues the source-level native-delivery implementation roadmap by verifying the native-delivery scorecard against shipped code, aligning packaging and documentation to the implemented native entrypoints, auditing any remaining bounded compatibility exceptions, and defining the final evidence-backed source-level native-product exit claim.

## Milestone Requirements

- [ ] **NDI-21**: The milestone verifies the source-level native-delivery scorecard against the shipped codebase and implemented entrypoints.
- [ ] **NDI-22**: The milestone aligns packaging, docs, and contributor guidance to the implemented native delivery surfaces instead of the retired command-tree story.
- [ ] **NDI-23**: The milestone audits any remaining native shims or true compatibility exceptions explicitly instead of hiding them inside a completion claim.
- [ ] **NDI-24**: The milestone defines the final source-level native-product exit claim truthfully against implemented evidence.

## Future Requirements

- The completed `18/18`, `6/6`, and `8/8` denominators must remain closed and must not be reinterpreted as implementation percentages.
- The implementation roadmap should only close at `6/6` when the native-product claim is backed by shipped source evidence, packaging truth, and explicit exception accounting.
- Any surviving shim or exception after this milestone must stay classified explicitly instead of silently preserving legacy ownership.
- Follow-on work after `v1.38` must use a new canonical denominator instead of extending the completed implementation roadmap implicitly.

## Out of Scope

- Reopening the completed historical, adapter-only, or planning denominators with a new meaning
- Claiming every legacy surface is deleted in source if bounded shims or explicit exceptions still remain
- Treating planning completion alone as proof of source-level native-product completion
- Broad new feature work unrelated to native-product verification, packaging truth, or compatibility exit

## Traceability

- `NDI-21` -> Phase 157
- `NDI-22` -> Phase 158
- `NDI-23` -> Phase 159
- `NDI-24` -> Phase 160
