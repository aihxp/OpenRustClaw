# Phase 129 Summary

The native-delivery roadmap now defines the first retirement inventory for the legacy command tree. The main bootstrap hotspot, the top-level binary routing path, and the major operator command families now have explicit retirement states instead of remaining open-ended future cleanup.

## Evidence

- `.planning/codebase/NATIVE-DELIVERY-LAYER-ROADMAP.md`
- `cargo metadata --no-deps --format-version 1`
- `wc -l crates/cli/src/{main.rs,commands/mod.rs} crates/cli/src/commands/{start.rs,inspect.rs,skills.rs,runtime.rs,mobile.rs,voice_runtime.rs,orchestrate.rs,browser.rs,onboard.rs,services.rs,channels.rs,control.rs,schedule.rs,memory.rs,media.rs,tools.rs}`
