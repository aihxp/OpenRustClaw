# Phase 31 Verification

## Commands

```bash
cargo test -p openrustclaw-cli onboard -- --nocapture
cargo test -p openrustclaw-cli inspect -- --nocapture
cargo test -p openrustclaw-cli control_ui -- --nocapture
cargo test -p openrustclaw-cli doctor -- --nocapture
```

## Outcome

- passed

## Evidence

- onboarding prints a handoff derived from durable setup state
- `/control/setup/handoff` now exposes the same contract over the shipped operator API
- `/control/ui` now includes `Setup Handoff`
- getting-started docs and README now match the shipped setup contract
