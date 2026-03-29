# Requirements: v1.32 Native Delivery Layer: Native Product Exit Audit and Packaging

**Started:** 2026-03-28
**Historical greenfield baseline:** retired `18/18` ranked seam ledger complete, or `100%`
**Full-conversion roadmap baseline:** `6/6` milestones shipped, or `100%`
**Native-delivery roadmap baseline:** `7/8` milestones shipped, or about `88%`
**Target after shipment:** `8/8` milestones shipped, or `100%`

## Scope

This milestone closes the native-delivery program by defining the final scorecard, architecture-doc and packaging alignment, compatibility-exception audit, and explicit exit claim required to say the product path is cleanly native.

## Milestone Requirements

### Exit Scorecard

- [ ] **NDL-29**: The roadmap defines the final scorecard proving the main product entrypoints are native delivery surfaces.

### Docs and Packaging

- [ ] **NDL-30**: The roadmap defines the docs, packaging, and contributor-guidance updates required for the native architecture.

### Compatibility Exceptions

- [ ] **NDL-31**: The roadmap defines the final audit of any remaining compatibility shims or exceptions.

### Exit Claim

- [ ] **NDL-32**: The roadmap defines the explicit native-product exit claim and the conditions under which it is truthful.

## Future Requirements

- The milestone archive must state explicitly whether the repo can now claim clean native or greenfield delivery ownership.
- Any remaining compatibility shim after this milestone must be documented explicitly as an exception rather than implied away by the final claim.
- Packaging and contributor guidance must point operators and contributors at the native architecture rather than the retired command tree.
- The native-delivery roadmap should only close at `8/8` when the scorecard, docs or packaging alignment, exception audit, and exit claim are explicit end to end.

## Out of Scope

- Reopening the completed `18/18` or `6/6` denominators with a new meaning
- Starting a new follow-on roadmap before the native-delivery program is closed
- Claiming absolute perfection or zero exceptions if the final audit does not support that
- Hiding compatibility exceptions behind a marketing-style completion statement

## Traceability

- `NDL-29` -> Phase 133
- `NDL-30` -> Phase 134
- `NDL-31` -> Phase 135
- `NDL-32` -> Phase 136
