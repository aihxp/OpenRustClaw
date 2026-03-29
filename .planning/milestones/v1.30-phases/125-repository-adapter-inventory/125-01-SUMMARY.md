# Phase 125 Summary

The native-delivery roadmap now defines the repository and gateway adapter inventory for the persistence-heavy delivery families. Workspace layout, runtime state, registries, audit evidence, session history, and memory or channel persistence are no longer described as permanent command-local ownership.

## Evidence

- `.planning/codebase/NATIVE-DELIVERY-LAYER-ROADMAP.md`
- `cargo metadata --no-deps --format-version 1`
- `wc -l crates/cli/src/commands/{inspect.rs,skills.rs,runtime.rs,services.rs,channels.rs,memory.rs,control.rs}`
