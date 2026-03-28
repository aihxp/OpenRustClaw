# Plan 62-01 Summary: Extract a Bounded Control Route Family Behind an Application Service Seam

## Result

Passed. The `/control/self-hosted/product-mode` transition path now runs through `openrustclaw-app` instead of being orchestrated directly inside `crates/cli/src/commands/start.rs`.

## What Changed

- extended the self-hosted product-mode application lane with a control service that owns the transition-and-report use case
- kept `inspect.rs` as the workspace adapter that bridges persisted CLI product-mode storage into that new service boundary
- reduced `start.rs` to a thin HTTP adapter for the migrated route family instead of letting it call `self_hosted::transition_mode(...)` directly
- added focused regression coverage for the migrated transition path alongside the existing summary and Control UI checks

## Evidence

- `crates/app/src/self_hosted_product.rs`
- `crates/cli/src/commands/inspect.rs`
- `crates/cli/src/commands/start.rs`
- `cargo test -p openrustclaw-app -- --nocapture`
- `cargo test -p openrustclaw-cli self_hosted_product_mode_summary -- --nocapture`
- `cargo test -p openrustclaw-cli transition_self_hosted_product_mode_summary -- --nocapture`
- `cargo test -p openrustclaw-cli dashboard_includes_self_hosted_product_mode_panel -- --nocapture`
