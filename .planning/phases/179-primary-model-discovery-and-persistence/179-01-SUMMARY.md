# Phase 179 Summary

Onboarding now asks for an explicit primary task model instead of inheriting a hidden default. After provider setup succeeds, the model step can discover live provider catalogs, offer a discovered shortlist when available, fall back to a recommended manual default when discovery is unavailable, persist the final provider-model pair through the shared runtime switch lane, and store the selected model plus its provenance in durable setup state.

## Evidence

- `crates/cli/src/commands/onboard.rs`
- `crates/app/src/setup_lifecycle.rs`
- `crates/app/src/setup_handoff.rs`
- `crates/cli/src/commands/inspect.rs`
- `cargo test -p openrustclaw-cli test_build_model_shortlist_prefers_recommended_model -- --nocapture`
- `cargo test -p openrustclaw-cli test_persist_provider_path_selection_clears_prior_model_selection -- --nocapture`
- `cargo test -p openrustclaw-cli test_setup_state_round_trip_and_resume_detection -- --nocapture`
- `cargo test -p openrustclaw-cli switch_provider_updates_runtime_config_through_service_lane -- --nocapture`
- `cargo test -p openrustclaw-cli setup_handoff_summary_reports_degraded_setup_state -- --nocapture`
- `cargo test -p openrustclaw-app setup_handoff -- --nocapture`
