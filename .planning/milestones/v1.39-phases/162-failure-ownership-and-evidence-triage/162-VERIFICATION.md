# Verification 162: Failure Ownership and Evidence Triage

## Evidence Reviewed

- `cargo test -p openrustclaw-e2e-tests --test e2e_tests -- --nocapture`
- `cargo test -p openrustclaw-integration-tests --lib -- --nocapture`

## Result

Passed.

- App-lane blocking failures: none observed.
- Native-delivery blocking failures: none observed.
- Infrastructure blocking failures: none observed.
- Bounded legacy-exception blocking failures: none observed.
- Non-blocking signals: dead-code warnings in `crates/cli/src/commands/mobile.rs` and `crates/cli/src/commands/voice_runtime.rs`.
