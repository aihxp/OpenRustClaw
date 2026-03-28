# Plan 61-01 Summary: Migrate the Next Inspection Summary Family into the Application Lane

## Result

Passed. Self-hosted product-mode summary composition now runs through `openrustclaw-app` instead of being owned directly by `crates/cli/src/commands/inspect.rs`.

## What Changed

- added a self-hosted product-mode service in `openrustclaw-app`
- moved transition-target derivation and operator-facing detail composition into the new application-layer service
- kept `inspect.rs` as a bounded adapter that loads persisted product-mode state, warnings, and transition receipts from the existing CLI module

## Evidence

- `crates/app/src/self_hosted_product.rs`
- `crates/cli/src/commands/inspect.rs`
- `cargo test -p openrustclaw-app -- --nocapture`
- `cargo test -p openrustclaw-cli self_hosted_product_mode_summary -- --nocapture`
- `cargo test -p openrustclaw-cli dashboard_includes_self_hosted_product_mode_panel -- --nocapture`
