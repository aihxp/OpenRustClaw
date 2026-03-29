# Phase 117 Summary

The native-delivery roadmap now defines the remaining large operator command families as explicit native CLI delivery targets over app ports. Browser, orchestration, mobile, voice runtime, onboarding, skills, and self-hosted flows now have named delivery ownership instead of defaulting conceptually to the legacy command tree.

## Evidence

- `.planning/codebase/NATIVE-DELIVERY-LAYER-ROADMAP.md`
- `cargo metadata --no-deps --format-version 1`
- `wc -l crates/cli/src/commands/{browser.rs,orchestrate.rs,mobile.rs,voice_runtime.rs,onboard.rs,skills.rs}`
