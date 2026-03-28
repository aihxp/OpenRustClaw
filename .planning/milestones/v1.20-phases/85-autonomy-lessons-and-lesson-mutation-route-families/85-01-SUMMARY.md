# Phase 85 Summary

## What Changed

- Added `crates/app/src/autonomy_lessons_control.rs` as the application-owned lesson-control lane.
- Updated `crates/cli/src/commands/start.rs` so the autonomy lesson summary, list, create, and deactivate handlers now adapt into that service.
- Added focused app-layer and route-family tests for the shipped lesson-control surfaces.

## Outcome

OpenRustClaw now routes the autonomy lesson-control route family through `openrustclaw-app`, while `start.rs` remains the HTTP adapter for the shipped lesson-control contract.
