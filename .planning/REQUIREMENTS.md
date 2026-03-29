# Requirements: v1.26 Native Delivery Layer: Control, MCP, and Gateway Delivery

**Started:** 2026-03-28
**Historical greenfield baseline:** retired `18/18` ranked seam ledger complete, or `100%`
**Full-conversion roadmap baseline:** `6/6` milestones shipped, or `100%`
**Native-delivery roadmap baseline:** `1/8` milestones shipped, or about `13%`
**Target after shipment:** `2/8` milestones shipped, or `25%`

## Scope

This milestone starts the first real execution slice of the native-delivery roadmap by defining the native control HTTP layer, the native MCP server delivery path, the gateway bootstrap split out of `start.rs`, and the Control UI serving alignment needed to begin retiring the legacy bootstrap hotspot.

## Milestone Requirements

### Control Delivery

- [ ] **NDL-05**: The roadmap defines the native control HTTP delivery layer over app ports and makes the `start.rs` control-route replacement path explicit.

### MCP Delivery

- [ ] **NDL-06**: The roadmap defines the native MCP server delivery path, including tool-catalog and invocation ownership outside the legacy command tree.

### Gateway Bootstrap

- [ ] **NDL-07**: The roadmap defines the gateway or server bootstrap split out of `start.rs` and leaves a real `start.rs` retirement slice.

### Control UI Alignment

- [ ] **NDL-08**: The roadmap aligns Control UI serving and wiring to the native gateway-delivery path without weakening compatibility for the shipped UI contract.

## Future Requirements

- Later milestones must implement the native control and MCP delivery layers described here.
- `start.rs` must stop being the primary bootstrap hotspot once the gateway and MCP delivery replacements ship.
- Control UI transport must eventually use the native gateway delivery layer as its primary serving path.
- The native-delivery roadmap should only advance to `2/8` when the control, MCP, gateway, and UI alignment path is explicit enough to implement directly.

## Out of Scope

- Full deletion of `start.rs` during `v1.26`
- Replacing all CLI delivery surfaces in the same milestone
- Worker-host and repository-adapter implementation work that belongs to later native-delivery milestones
- Reopening the completed `18/18` or `6/6` denominators with a new meaning

## Traceability

- `NDL-05` -> Phase 109
- `NDL-06` -> Phase 110
- `NDL-07` -> Phase 111
- `NDL-08` -> Phase 112
