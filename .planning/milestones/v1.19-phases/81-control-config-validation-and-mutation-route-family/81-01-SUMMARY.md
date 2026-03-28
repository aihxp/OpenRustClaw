# Phase 81 Summary

## What Changed

- Added `crates/app/src/control_config.rs` as the application-owned control config service lane.
- Updated `crates/cli/src/commands/start.rs` so the `/control/config` read, validate, and update handlers now adapt into that service.
- Added focused app-layer and route-family tests to preserve the current control config contract.

## Outcome

OpenRustClaw now routes the control config route family through `openrustclaw-app`, while `start.rs` remains the HTTP adapter around config-path wiring and response mapping.
