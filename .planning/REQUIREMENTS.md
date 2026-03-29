# Requirements: v1.28 Native Delivery Layer: CLI Operator Commands II and UI-Adjacent Flows

**Started:** 2026-03-28
**Historical greenfield baseline:** retired `18/18` ranked seam ledger complete, or `100%`
**Full-conversion roadmap baseline:** `6/6` milestones shipped, or `100%`
**Native-delivery roadmap baseline:** `3/8` milestones shipped, or about `38%`
**Target after shipment:** `4/8` milestones shipped, or `50%`

## Scope

This milestone continues the native-delivery CLI replacement work by defining the remaining large operator-facing command families, the secondary utility families, the command-family dependency cleanup, and the UI-adjacent entrypoint alignment needed for the second CLI slice.

## Milestone Requirements

### Large Operator Families

- [ ] **NDL-13**: The roadmap defines native delivery for browser, orchestration, mobile, voice runtime, onboarding, skills, and self-hosted operator flows.

### Secondary Operator and Utility Families

- [ ] **NDL-14**: The roadmap defines native delivery for channels, services, schedule, tools, media, memory, and adjacent secondary utility command families.

### Dependency Removal

- [ ] **NDL-15**: The roadmap defines how command-to-command orchestration dependencies between remaining CLI families are removed or avoided.

### UI-Adjacent Alignment

- [ ] **NDL-16**: The roadmap aligns UI-adjacent operator entrypoints to native delivery paths so those surfaces stop depending conceptually on legacy command ownership.

## Future Requirements

- Later milestones must implement the second CLI operator slice described here.
- Remaining CLI families must stop depending on legacy file-to-file orchestration once native delivery modules exist.
- UI-adjacent operator surfaces must point to native entrypoints rather than legacy command ownership as the replacement work lands.
- The native-delivery roadmap should only advance to `4/8` when the second CLI replacement slice is explicit enough to implement directly.

## Out of Scope

- Full deletion of all remaining operator command files during `v1.28`
- Worker-host or repository-adapter implementation work that belongs to later native-delivery milestones
- Reopening the completed `18/18` or `6/6` denominators with a new meaning
- Claiming complete CLI legacy exit before the remaining delivery families and compatibility rules are implemented

## Traceability

- `NDL-13` -> Phase 117
- `NDL-14` -> Phase 118
- `NDL-15` -> Phase 119
- `NDL-16` -> Phase 120
