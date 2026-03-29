# Requirements: v1.34 Native Delivery Implementation: Native CLI Dispatch and Core Operator Paths

**Started:** 2026-03-28
**Historical greenfield baseline:** retired `18/18` ranked seam ledger complete, or `100%`
**Full-conversion roadmap baseline:** `6/6` milestones shipped, or `100%`
**Native-delivery planning roadmap baseline:** `8/8` milestones shipped, or `100%`
**Native-delivery implementation roadmap baseline:** `1/6` milestones shipped, or about `17%`
**Target after shipment:** `2/6` milestones shipped, or about `33%`

## Scope

This milestone continues the source-level native-delivery implementation roadmap by defining the first native CLI dispatch slice, the first assistant/session and inspection operator-path slice, the first control/runtime CLI handoff, and the first compatibility-and-verification rules for the new CLI successor path.

## Milestone Requirements

### CLI Dispatch

- [ ] **NDI-5**: The roadmap defines the first native CLI dispatch slice that reduces top-level routing ownership in the legacy command tree.

### Core Operator Paths

- [ ] **NDI-6**: The roadmap defines the first assistant, chat, session, and inspect native CLI delivery slice.

### Control and Runtime Handoff

- [ ] **NDI-7**: The roadmap defines the first control and runtime native CLI handoff slice.

### Compatibility and Verification

- [ ] **NDI-8**: The roadmap defines the compatibility and verification rules for the first native CLI operator-path slice.

## Future Requirements

- Later milestones must continue native CLI, runtime-host, repository, retirement, and verification implementation through the same roadmap instead of inventing a new denominator.
- The completed `18/18`, `6/6`, and `8/8` denominators must remain closed and must not be silently reinterpreted as source-level implementation percentages.
- The implementation roadmap should only advance beyond `2/6` when the first native CLI successor slice is explicit enough to implement and verify.
- Any compatibility shim that survives the CLI handoff must stay bounded explicitly instead of preserving hidden routing ownership.

## Out of Scope

- Reopening the completed historical, adapter-only, or planning denominators with a new meaning
- Claiming the native CLI dispatch and operator paths are already implemented in source before the implementation milestones ship
- Retiring the whole command tree in the same milestone that starts the first native CLI slice
- Treating planning-only completion as proof of source-level CLI ownership

## Traceability

- `NDI-5` -> Phase 141
- `NDI-6` -> Phase 142
- `NDI-7` -> Phase 143
- `NDI-8` -> Phase 144
