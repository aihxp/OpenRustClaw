# Requirements: v1.31 Native Delivery Layer: Legacy Module Retirement and Compatibility Shutdown

**Started:** 2026-03-28
**Historical greenfield baseline:** retired `18/18` ranked seam ledger complete, or `100%`
**Full-conversion roadmap baseline:** `6/6` milestones shipped, or `100%`
**Native-delivery roadmap baseline:** `6/8` milestones shipped, or `75%`
**Target after shipment:** `7/8` milestones shipped, or about `88%`

## Scope

This milestone continues the native-delivery program by defining the legacy-module retirement inventory, the compatibility-shim and delete boundaries, the `main.rs` retirement path, and the guardrails needed to prove the legacy command tree is no longer the main product path.

## Milestone Requirements

### Legacy Module Retirement

- [ ] **NDL-25**: The roadmap defines how legacy command modules leave the main product path.

### Compatibility Shutdown

- [ ] **NDL-26**: The roadmap defines how remaining live legacy surfaces become thin compatibility shims or hard deletes.

### Bootstrap Retirement

- [ ] **NDL-27**: The roadmap defines how `main.rs` shrinks to binary bootstrap only or is replaced entirely.

### Guardrails and Verification

- [ ] **NDL-28**: The roadmap defines guardrails and verification proving new product entrypoints no longer depend on retired delivery files.

## Future Requirements

- Later milestones must implement the legacy-module retirement and compatibility-shutdown paths described here.
- Retired legacy modules must stop regaining product-path ownership once native entrypoints and shims are defined.
- Any remaining compatibility shims after this milestone must point explicitly to native delivery paths instead of preserving hidden orchestration or persistence ownership.
- The native-delivery roadmap should only advance beyond `7/8` when the final native-product exit audit and packaging slice starts.

## Out of Scope

- The final native-product exit audit and packaging work that belongs to `v1.32`
- Reopening the completed `18/18` or `6/6` denominators with a new meaning
- Claiming full native-product exit before the final audit milestone closes
- Treating undocumented legacy paths as implicitly retired

## Traceability

- `NDL-25` -> Phase 129
- `NDL-26` -> Phase 130
- `NDL-27` -> Phase 131
- `NDL-28` -> Phase 132
