# Phase 177 Summary

Onboarding now treats provider choice as a provider-path decision instead of a single implicit API-key lane. The model setup step persists the selected provider and access mode, restores them on resume, and carries that state through setup lifecycle and handoff reporting so later phases can build verification and explicit model selection on top of a truthful baseline.

## Evidence

- `crates/cli/src/commands/onboard.rs`
- `crates/app/src/setup_lifecycle.rs`
- `crates/app/src/setup_handoff.rs`
- `crates/cli/src/commands/inspect.rs`
- `cargo test -p openrustclaw-cli onboarding -- --nocapture`
- `cargo test -p openrustclaw-app setup_handoff -- --nocapture`
- `cargo test -p openrustclaw-cli setup_handoff_summary_reports_degraded_setup_state -- --nocapture`
- `cargo test -p openrustclaw-app setup_lifecycle -- --nocapture`
