# Requirements: v1.30 Native Delivery Layer: Repositories and Integration Adapters

**Started:** 2026-03-28
**Historical greenfield baseline:** retired `18/18` ranked seam ledger complete, or `100%`
**Full-conversion roadmap baseline:** `6/6` milestones shipped, or `100%`
**Native-delivery roadmap baseline:** `5/8` milestones shipped, or about `63%`
**Target after shipment:** `6/8` milestones shipped, or `75%`

## Scope

This milestone continues the native-delivery program by defining repository and gateway adapter ownership for persistence-heavy delivery families and external integrations so command-local file helpers and side-effect wiring can leave the legacy command modules.

## Milestone Requirements

### Repository Adapter Inventory

- [ ] **NDL-21**: The roadmap defines repository and gateway adapters for sqlite, workspace files, audit logs, runtime config, compiled-skill cache, and registries.

### Integration Gateway Boundaries

- [ ] **NDL-22**: The roadmap defines infrastructure boundaries for channel providers and external services.

### App Port to Repository Contracts

- [ ] **NDL-23**: The roadmap defines how app services depend on adapter traits or repositories instead of command-local file helpers.

### Adapter Verification and Exit Rules

- [ ] **NDL-24**: The roadmap defines direct repository and adapter verification plus ownership-exit criteria for removing command-local persistence verification.

## Future Requirements

- Later milestones must implement the repository and gateway adapters described here.
- Persistence layout, sqlite access, registries, and external service wiring must stop depending on command-local helper ownership once the native adapter path ships.
- Any compatibility shims retained after this milestone must point to repository-backed and gateway-backed native paths instead of preserving command-owned persistence logic.
- The native-delivery roadmap should only advance beyond `6/8` when legacy-module retirement work starts replacing live command modules with shims or deletions.

## Out of Scope

- Full deletion of legacy command modules during `v1.30`
- Native product exit audit or packaging work that belongs to later milestones
- Reopening the completed `18/18` or `6/6` denominators with a new meaning
- Claiming complete persistence or integration legacy exit before repository and gateway adapters are implemented

## Traceability

- `NDL-21` -> Phase 125
- `NDL-22` -> Phase 126
- `NDL-23` -> Phase 127
- `NDL-24` -> Phase 128
