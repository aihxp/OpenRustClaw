# Plan 65-01 Summary: Extract the Enterprise Admin Aggregation Family

## Result

Passed. The enterprise admin aggregation now runs through `openrustclaw-app` instead of being composed directly inside `crates/cli/src/commands/inspect.rs`.

## What Changed

- added an enterprise admin aggregation service to `openrustclaw-app`
- moved enterprise admin status, detail, and supervision composition behind that shared service
- kept `inspect.rs` as the workspace adapter that loads enterprise access, policy, autonomy, and active-run state into the new service
- preserved the shipped `/control/enterprise/admin` and Control UI contract while shrinking `inspect.rs` ownership of enterprise aggregation logic

## Evidence

- `crates/app/src/enterprise_admin.rs`
- `crates/cli/src/commands/inspect.rs`
- `cargo test -p openrustclaw-app enterprise_admin -- --nocapture`
- `cargo test -p openrustclaw-cli enterprise_admin_summary_combines_access_policy_and_supervision -- --nocapture`
