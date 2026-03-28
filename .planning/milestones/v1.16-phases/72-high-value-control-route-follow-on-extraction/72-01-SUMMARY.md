# Plan 72-01 Summary: Extract the Runtime Vault Control Route Family

## Result

Passed. The `/control/runtime/vault` route family now runs through `openrustclaw-app` instead of hand-assembling route behavior directly inside `crates/cli/src/commands/start.rs`.

## What Changed

- added a runtime vault control service to `openrustclaw-app`
- moved vault route-family behavior and response shaping behind that shared service
- kept `start.rs` as the HTTP adapter that maps runtime vault requests into the control service
- preserved the shipped `/control/runtime/vault` API contract while reducing route-local coupling around the runtime vault seam introduced earlier in the milestone

## Evidence

- `crates/app/src/runtime_vault_control.rs`
- `crates/cli/src/commands/start.rs`
- `cargo test -p openrustclaw-app runtime_vault_control -- --nocapture`
- `cargo test -p openrustclaw-cli runtime_vault_route_family_uses_service_lane -- --nocapture`
