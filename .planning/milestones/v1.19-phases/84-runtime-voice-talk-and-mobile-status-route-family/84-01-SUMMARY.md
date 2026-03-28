# Phase 84 Summary

## What Changed

- Added `crates/app/src/operator_status_control.rs` as the application-owned runtime, voice, talk, and mobile status lane.
- Updated `crates/cli/src/commands/start.rs` so the read-heavy runtime, voice, talk, and mobile status handlers now adapt into that service.
- Added focused app-layer and route-family tests for the shipped status surfaces.

## Outcome

OpenRustClaw now routes the remaining read-heavy runtime, voice, talk, and mobile status orchestration through `openrustclaw-app`, while `start.rs` remains the HTTP adapter for the shipped control-plane status route family.
