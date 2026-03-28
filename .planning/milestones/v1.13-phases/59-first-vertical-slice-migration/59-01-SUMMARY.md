# Plan 59-01 Summary: Migrate the First Proving Slice into the Greenfield Lane

## Result

Passed. Setup handoff reporting now flows through `openrustclaw-app` instead of being composed directly inside `crates/cli/src/commands/inspect.rs`.

## What Changed

- `openrustclaw-cli` now depends on `openrustclaw-app`.
- `inspect.rs` now acts as an adapter that loads onboarding setup state and maps it into `openrustclaw_app::setup_handoff::SetupHandoffService`.
- The runtime route and Control UI report contract stayed stable, so the migrated slice remains operator-visible without a dashboard or route rewrite.

## Evidence

- `crates/cli/Cargo.toml`
- `crates/cli/src/commands/inspect.rs`
- `cargo test -p openrustclaw-app -- --nocapture`
- `cargo test -p openrustclaw-cli setup_handoff_summary -- --nocapture`
- `cargo test -p openrustclaw-cli dashboard_includes_setup_handoff_panel -- --nocapture`
