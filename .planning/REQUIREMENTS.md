# Requirements: v1.37 Native Delivery Implementation: Legacy Command Tree Retirement

**Started:** 2026-03-28
**Historical greenfield baseline:** retired `18/18` ranked seam ledger complete, or `100%`
**Full-conversion roadmap baseline:** `6/6` milestones shipped, or `100%`
**Native-delivery planning roadmap baseline:** `8/8` milestones shipped, or `100%`
**Native-delivery implementation roadmap baseline:** `4/6` milestones shipped, or about `67%`
**Target after shipment:** `5/6` milestones shipped, or about `83%`

## Scope

This milestone continues the source-level native-delivery implementation roadmap by defining the first legacy module retirement inventory and successor ownership slice, the first delete-or-shim boundaries for superseded command-tree hotspots, the first `main.rs` bootstrap retirement path, and the first guardrails plus compatibility-and-verification rules for legacy command-tree retirement.

## Milestone Requirements

### Retirement Inventory

- [ ] **NDI-17**: The roadmap defines the first legacy module retirement inventory and successor ownership slice for superseded command-tree hotspots.

### Delete or Shim Boundaries

- [ ] **NDI-18**: The roadmap defines the first delete-or-shim boundaries plus `main.rs` bootstrap retirement path over native ownership.

### Guardrails

- [ ] **NDI-19**: The roadmap defines the first guardrails needed to keep retired command-tree hotspots out of the main product path.

### Compatibility and Verification

- [ ] **NDI-20**: The roadmap defines the direct verification and compatibility rules for the first legacy command-tree retirement slice.

## Future Requirements

- Later milestones must continue final native-product verification and packaging implementation through the same roadmap instead of inventing a new denominator.
- The completed `18/18`, `6/6`, and `8/8` denominators must remain closed and must not be silently reinterpreted as source-level implementation percentages.
- The implementation roadmap should only advance beyond `5/6` when the first legacy retirement successor slice is explicit enough to implement and verify.
- Any compatibility shim that survives the retirement handoff must stay bounded explicitly instead of preserving hidden ownership in superseded command-tree modules.

## Out of Scope

- Reopening the completed historical, adapter-only, or planning denominators with a new meaning
- Claiming the legacy command tree is already retired in source before the implementation milestones ship
- Final native-product exit verification that belongs to the last implementation milestone
- Treating planning-only completion as proof of source-level legacy retirement

## Traceability

- `NDI-17` -> Phase 153
- `NDI-18` -> Phase 154, Phase 155
- `NDI-19` -> Phase 156
- `NDI-20` -> Phase 156
