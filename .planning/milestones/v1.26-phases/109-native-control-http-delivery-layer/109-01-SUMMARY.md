# Phase 109 Summary

The native-delivery roadmap now defines control HTTP delivery as a gateway-native concern over `ControlPlanePort` instead of treating `start.rs` as the permanent route-registration home. The milestone leaves a bounded compatibility slice for startup forwarding while making future control-route ownership explicit.

## Evidence

- `.planning/codebase/NATIVE-DELIVERY-LAYER-ROADMAP.md`
- `cargo metadata --no-deps --format-version 1`
- `wc -l crates/cli/src/commands/start.rs crates/gateway/src/lib.rs`
