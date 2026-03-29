# Phase 97 Summary

Onboarding, repair, and resume orchestration now compose through `openrustclaw-app::setup_lifecycle`. `crates/cli/src/commands/onboard.rs` remains responsible for setup-state persistence, workspace reads, and bounded runtime probes, but setup-profile defaults, step planning, repair-plan derivation, and handoff shaping are now application-owned.

## Evidence

- `crates/app/src/setup_lifecycle.rs`
- `crates/cli/src/commands/onboard.rs`
- `cargo test -p openrustclaw-app --lib -- --nocapture`
- `cargo check -p openrustclaw-cli --lib`
