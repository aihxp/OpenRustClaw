# Phase 126 Summary

The native-delivery roadmap now defines explicit gateway boundaries for channel providers and external services. External side effects are no longer treated as command-local helper clusters, which leaves a concrete gateway-backed implementation path for later work.

## Evidence

- `.planning/codebase/NATIVE-DELIVERY-LAYER-ROADMAP.md`
- `cargo metadata --no-deps --format-version 1`
- `wc -l crates/cli/src/commands/{skills.rs,runtime.rs,services.rs,channels.rs,control.rs}`
