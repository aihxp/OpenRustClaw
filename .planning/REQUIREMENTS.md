# Requirements: v1.33 Native Delivery Implementation: Gateway and MCP Successor Entry Points

**Started:** 2026-03-28
**Historical greenfield baseline:** retired `18/18` ranked seam ledger complete, or `100%`
**Full-conversion roadmap baseline:** `6/6` milestones shipped, or `100%`
**Native-delivery planning roadmap baseline:** `8/8` milestones shipped, or `100%`
**Native-delivery implementation roadmap baseline:** `0/6` milestones shipped, or `0%`
**Target after shipment:** `1/6` milestones shipped, or about `17%`

## Scope

This milestone starts the source-level native-delivery implementation roadmap by defining the first real successor entrypoint slice for control HTTP, MCP startup, Control UI serving, and the first bounded `start.rs` ownership removal.

## Milestone Requirements

### Gateway Successor

- [ ] **NDI-1**: The roadmap defines the first source-level native control HTTP bootstrap slice over `openrustclaw-gateway`.

### MCP Successor

- [ ] **NDI-2**: The roadmap defines the first source-level native MCP bootstrap slice over `openrustclaw-mcp`.

### Startup Handoff

- [ ] **NDI-3**: The roadmap defines the Control UI serving and startup handoff needed to reduce `start.rs` ownership truthfully.

### Compatibility and Verification

- [ ] **NDI-4**: The roadmap defines the first source-level compatibility and verification rules for the gateway and MCP successor entrypoints.

## Future Requirements

- Later milestones must implement the successor gateway, MCP, CLI, runtime-host, repository, and retirement slices defined by the new implementation roadmap.
- The completed `18/18`, `6/6`, and `8/8` denominators must remain closed and must not be silently reinterpreted as source-level implementation percentages.
- The implementation roadmap should only advance beyond `1/6` when the first source-level successor entrypoint slice is explicit enough to implement and verify.
- Compatibility coverage must stay explicit whenever `start.rs` continues to exist during the successor-entrypoint transition.

## Out of Scope

- Reopening the completed historical, adapter-only, or planning denominators with a new meaning
- Claiming the control and MCP successor entrypoints are already implemented in source before the implementation milestones ship
- Retiring the whole command tree in the same milestone that starts the first gateway and MCP successor slice
- Treating planning-only completion as proof of source-level native entrypoint ownership

## Traceability

- `NDI-1` -> Phase 137
- `NDI-2` -> Phase 138
- `NDI-3` -> Phase 139
- `NDI-4` -> Phase 140
