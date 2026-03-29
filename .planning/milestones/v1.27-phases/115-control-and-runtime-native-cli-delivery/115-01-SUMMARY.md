# Phase 115 Summary

The native-delivery roadmap now defines explicit CLI ownership for control and runtime entrypoints over `ControlPlanePort` and `RuntimeOperationsPort`. That keeps the native CLI roadmap honest by covering operator command flows that still default to legacy command hubs even after the control HTTP story exists.

## Evidence

- `.planning/codebase/NATIVE-DELIVERY-LAYER-ROADMAP.md`
- `cargo metadata --no-deps --format-version 1`
- `wc -l crates/cli/src/commands/{runtime.rs,control.rs}`
