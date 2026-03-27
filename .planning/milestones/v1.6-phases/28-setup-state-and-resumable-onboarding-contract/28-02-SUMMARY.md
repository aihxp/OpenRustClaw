# Plan 28-02 Summary: Resume Onboarding with Standard, Advanced, and Custom Paths

## What Changed

- Replaced the old transient quick path with explicit `Standard`, `Advanced`, and `Custom` setup paths.
- Added resume behavior when an unfinished setup-state manifest already exists.
- Custom setup now persists the selected step list and resume only runs the unfinished steps instead of replaying the whole wizard blindly.

## Why It Matters

The setup-depth choice is now explicit and durable. Operators can start with a standard path, opt into advanced or custom control when needed, and re-enter onboarding without losing the intended setup flow.

## Verification Notes

- `cargo test -p openrustclaw-cli onboard -- --nocapture`
