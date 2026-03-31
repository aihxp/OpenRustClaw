# Phase 178 Summary

Onboarding now records live provider verification as durable setup evidence instead of collapsing failures into an opaque blocked state. The model lane keeps using the existing runtime health scan, but it now persists verification metadata such as issue kind, verification stage, and a targeted recovery action so resume, repair, inspect, and handoff surfaces can explain whether readiness failed because of auth, local runtime reachability, model availability, billing or quota, rate limits, or generic provider reachability.

## Evidence

- `crates/cli/src/commands/onboard.rs`
- `crates/app/src/setup_lifecycle.rs`
- `crates/app/src/setup_handoff.rs`
- `crates/cli/src/commands/inspect.rs`
- `cargo test -p openrustclaw-cli onboarding -- --nocapture`
- `cargo test -p openrustclaw-cli setup_handoff_summary_reports_degraded_setup_state -- --nocapture`
- `cargo test -p openrustclaw-cli test_validate_provider_bootstrap_classifies_local_runtime_missing -- --nocapture`
- `cargo test -p openrustclaw-cli test_mark_setup_state_step_blocked_prefers_provider_verification_action -- --nocapture`
- `cargo test -p openrustclaw-app setup_lifecycle -- --nocapture`
- `cargo test -p openrustclaw-app setup_handoff -- --nocapture`
