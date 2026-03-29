# Requirements: v1.29 Native Delivery Layer: Runtime Hosts and Background Workers

**Started:** 2026-03-28
**Historical greenfield baseline:** retired `18/18` ranked seam ledger complete, or `100%`
**Full-conversion roadmap baseline:** `6/6` milestones shipped, or `100%`
**Native-delivery roadmap baseline:** `4/8` milestones shipped, or `50%`
**Target after shipment:** `5/8` milestones shipped, or about `63%`

## Scope

This milestone continues the native-delivery program by defining dedicated runtime-host and background-worker entrypoints, the runtime startup boundaries, the worker boot contracts, and the legacy startup ownership removal needed for the next execution slice.

## Milestone Requirements

### Runtime Host Entry Points

- [ ] **NDL-17**: The roadmap defines dedicated runtime-host and background-worker entrypoints over app ports.

### Startup Boundaries

- [ ] **NDL-18**: The roadmap defines service-manager, probe-runner, runtime-maintenance, and scheduler startup boundaries for native runtime-host delivery.

### Worker Boot Contracts

- [ ] **NDL-19**: The roadmap aligns mobile, voice, and orchestration worker boot contracts with native runtime-host delivery.

### Legacy Startup Ownership

- [ ] **NDL-20**: The roadmap defines how legacy command ownership over worker lifecycle startup is removed or reduced.

## Future Requirements

- Later milestones must implement the native runtime-host and background-worker entrypoints described here.
- Worker startup must stop depending on legacy command-layer bootstraps once the native runtime-host path ships.
- Any compatibility shims retained after this milestone must point to native runtime-host entrypoints instead of preserving legacy startup ownership.
- The native-delivery roadmap should only advance to `5/8` when the runtime-host replacement slice is explicit enough to implement directly.

## Out of Scope

- Full deletion of all legacy startup paths during `v1.29`
- Repository-adapter implementation work that belongs to later native-delivery milestones
- Reopening the completed `18/18` or `6/6` denominators with a new meaning
- Claiming complete runtime-host legacy exit before the native entrypoints and compatibility rules are implemented

## Traceability

- `NDL-17` -> Phase 121
- `NDL-18` -> Phase 122
- `NDL-19` -> Phase 123
- `NDL-20` -> Phase 124
