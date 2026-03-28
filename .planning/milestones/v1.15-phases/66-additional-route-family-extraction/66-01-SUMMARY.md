# Plan 66-01 Summary: Extract the Enterprise Access Route Family

## Result

Passed. The enterprise access write route family now runs through `openrustclaw-app` instead of being orchestrated directly inside `crates/cli/src/commands/start.rs`.

## What Changed

- added an enterprise access control service to `openrustclaw-app`
- moved enterprise access bootstrap, operator upsert, and governance-rule upsert orchestration behind that shared service
- kept `start.rs` as the workspace and HTTP adapter that accepts payloads, records operator tool results, and returns the same enterprise access summary contract
- preserved the shipped `/control/enterprise/access/bootstrap`, `/control/enterprise/access/operators`, and `/control/enterprise/governance/rules` runtime surface while shrinking `start.rs` ownership of enterprise mutation-and-report logic

## Evidence

- `crates/app/src/enterprise_access_control.rs`
- `crates/cli/src/commands/start.rs`
- `cargo test -p openrustclaw-app enterprise_access_control -- --nocapture`
- `cargo test -p openrustclaw-cli enterprise_access_summary_reports_bootstrap_state_and_operator_counts -- --nocapture`
- `cargo test -p openrustclaw-cli protected_scope_classifies_sensitive_routes -- --nocapture`
