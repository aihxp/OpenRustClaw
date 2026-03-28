# Plan 75-01 Summary: Extract the Runtime Maintenance Planning Seam

## Result

Passed. Runtime upgrade, self-update, and rollback planning now run through `openrustclaw-app` instead of being orchestrated directly inside `crates/cli/src/commands/runtime.rs`.

## What Changed

- added a runtime maintenance-planning service to `openrustclaw-app`
- moved blocker detection and operator-step generation for upgrade, self-update, and rollback planning behind that shared service
- kept `runtime.rs` as the adapter that loads runtime status, health, reload-state, service-manager state, lock state, and artifact metadata

## Evidence

- `crates/app/src/runtime_maintenance_planning.rs`
- `crates/cli/src/commands/runtime.rs`
- `cargo test -p openrustclaw-app runtime_maintenance_planning -- --nocapture`
- `cargo test -p openrustclaw-cli runtime_maintenance_plans_use_service_lane -- --nocapture`
