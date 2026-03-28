# Plan 76-01 Summary: Extract the Greenfield Progress Surface and Runtime Maintenance Summary Route

## Result

Passed. OpenRustClaw now ships a `/control/runtime/maintenance` surface backed by `openrustclaw-app`, and the canonical greenfield inventory now reports the real post-milestone score instead of the original Phase 73 baseline.

## What Changed

- updated the ranked seam inventory to mark the Phase 74, 75, and 76 seams as migrated, leaving one remaining ranked seam
- added a runtime-maintenance control service to `openrustclaw-app` plus a shared greenfield progress summary helper
- wired `/control/runtime/maintenance` through that service so the shipped route reports the canonical completion score, remaining queue, and maintenance route hints

## Evidence

- `crates/app/src/greenfield_progress.rs`
- `crates/app/src/runtime_maintenance_control.rs`
- `crates/cli/src/commands/inspect.rs`
- `crates/cli/src/commands/start.rs`
- `.planning/codebase/GREENFIELD-INVENTORY.md`
- `cargo test -p openrustclaw-app greenfield_progress -- --nocapture`
- `cargo test -p openrustclaw-app runtime_maintenance_control -- --nocapture`
- `cargo test -p openrustclaw-cli greenfield_progress_summary_reports_current_inventory_score -- --nocapture`
- `cargo test -p openrustclaw-cli runtime_maintenance_route_family_uses_service_lane -- --nocapture`
