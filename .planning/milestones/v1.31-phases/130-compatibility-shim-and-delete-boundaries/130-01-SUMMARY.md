# Phase 130 Summary

The native-delivery roadmap now defines explicit shim-versus-delete boundaries for still-live legacy surfaces. Compatibility is no longer described as an open-ended excuse for the old command tree to remain in the product path once native entrypoints exist.

## Evidence

- `.planning/codebase/NATIVE-DELIVERY-LAYER-ROADMAP.md`
- `.planning/ROADMAP.md`
- `wc -l crates/cli/src/{main.rs,commands/mod.rs} crates/cli/src/commands/{start.rs,inspect.rs,skills.rs,runtime.rs,mobile.rs,voice_runtime.rs,orchestrate.rs,browser.rs,onboard.rs,services.rs,channels.rs,control.rs,schedule.rs,memory.rs,media.rs,tools.rs}`
