# Phase 127 Summary

The native-delivery roadmap now aligns the major app-port families directly to repositories and gateway adapters. Runtime, skills, inspection, control, channel, and memory-facing services are no longer described as depending on command-local filesystem, sqlite, or registry helpers.

## Evidence

- `.planning/codebase/NATIVE-DELIVERY-LAYER-ROADMAP.md`
- `cargo metadata --no-deps --format-version 1`
- `wc -l crates/cli/src/commands/{inspect.rs,skills.rs,runtime.rs,services.rs,channels.rs,memory.rs,control.rs}`
