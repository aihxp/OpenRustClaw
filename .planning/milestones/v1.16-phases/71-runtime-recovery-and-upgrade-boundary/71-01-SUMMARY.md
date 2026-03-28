# Plan 71-01 Summary: Extract the Runtime Reload Planning Lane

## Result

Passed. The runtime reload-plan seam now runs through `openrustclaw-app` instead of being orchestrated directly inside `crates/cli/src/commands/runtime.rs`.

## What Changed

- added a runtime reload-planning service to `openrustclaw-app`
- moved snapshot comparison, restart classification, and reload-plan result shaping behind that shared service
- kept `runtime.rs` as the adapter that captures runtime snapshots and loads the applied reload-state artifact
- preserved the shipped runtime reload-plan contract used by both operator-facing runtime APIs and higher-level recovery summaries

## Evidence

- `crates/app/src/runtime_reload_planning.rs`
- `crates/cli/src/commands/runtime.rs`
- `cargo test -p openrustclaw-app runtime_reload_planning -- --nocapture`
- `cargo test -p openrustclaw-cli runtime_reload_plan_uses_service_lane -- --nocapture`
