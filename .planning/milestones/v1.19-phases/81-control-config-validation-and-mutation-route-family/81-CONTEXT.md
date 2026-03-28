# Phase 81: Control Config Validation and Mutation Route Family - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning

## Phase Boundary

Move the remaining `/control/config`, `/control/config/validate`, and `/control/config/update` orchestration out of `start.rs` so the control-plane config path becomes another bounded application-owned route family.

## Decisions

- The extracted seam should preserve the current control config JSON contract and status-code behavior instead of redesigning the route surface.
- `start.rs` should remain the HTTP adapter that wires payloads and maps app-service results into responses.
- The new application service should own config rendering, mutation-result shaping, and the stable route-family report shape.

## Existing Code Insights

- `control_config_handler` still loads the effective config inline from `start.rs`.
- `control_config_validate_handler` and `control_config_update_handler` still shape the validation and write result inline, including byte-count reporting.
- The seam is narrow enough to extract without reopening provider-switch, reload, or broader runtime mutation behavior.
