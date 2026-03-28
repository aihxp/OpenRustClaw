# Phase 82 Summary

## What Changed

- Added `crates/app/src/control_diagnostics.rs` as the application-owned diagnostics service lane.
- Updated `crates/cli/src/commands/start.rs` so diagnostics report collection and websocket event shaping now adapt into that service.
- Added focused app-layer and route-family tests for diagnostics report and live event behavior.

## Outcome

OpenRustClaw now routes diagnostics report collection and live event payload shaping through `openrustclaw-app`, while `start.rs` remains the transport adapter for the `/control/diagnostics` route family.
