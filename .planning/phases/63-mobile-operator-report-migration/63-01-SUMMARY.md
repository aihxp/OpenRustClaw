# Plan 63-01 Summary: Migrate the Next Mobile Operator Report into the Application Lane

## Result

Passed. The mobile node operator summary report now runs through `openrustclaw-app` instead of being composed directly inside `crates/cli/src/commands/mobile.rs`.

## What Changed

- added a mobile operator-report service to `openrustclaw-app`
- kept `mobile.rs` as the workspace adapter that loads node state, metrics, and recent activity from the existing mobile data helpers
- moved attention-signal derivation and report composition behind that application-layer service
- preserved the shipped `/control/mobile/nodes/{id}/summary` contract and added focused regression coverage for the migrated report path

## Evidence

- `crates/app/src/mobile_operator.rs`
- `crates/cli/src/commands/mobile.rs`
- `cargo test -p openrustclaw-app -- --nocapture`
- `cargo test -p openrustclaw-cli mobile_node_report_surfaces_attention_signals_through_app_lane -- --nocapture`
- `cargo test -p openrustclaw-cli dashboard_includes_mobile_operator_report_rendering -- --nocapture`
