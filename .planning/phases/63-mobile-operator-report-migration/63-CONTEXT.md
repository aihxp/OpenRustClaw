# Phase 63: Mobile Operator Report Migration - Context

**Gathered:** 2026-03-28
**Status:** Ready for execution

## Goal

Prove the greenfield lane on a second operator-visible surface by moving one mobile operator report into `openrustclaw-app`.

## What We Know

- The current `/control/mobile/nodes/{id}/summary` path is a real shipped operator surface consumed by Control UI and backed by `mobile::mobile_node_report_data(...)`.
- `mobile_node_report_data(...)` currently lives fully inside `crates/cli/src/commands/mobile.rs`, where it assembles the operator report and derives attention signals directly in the CLI module.
- This surface is bounded and already has one shipped dashboard rendering check, which makes it a safer migration target than a broader mobile command or sync mutation path.
- The underlying mobile persistence, runtime updates, and command flows can stay in `mobile.rs`; this phase is about report composition ownership.

## Constraints

- The runtime API and Control UI contract for the mobile node summary report must stay intact.
- This phase should move report composition, not mobile storage or command mutation semantics.
- Verification needs one focused assertion that the migrated report still surfaces operator attention signals truthfully.

## Implementation Direction

- add a mobile operator report service to `openrustclaw-app`
- keep `mobile.rs` as the workspace adapter that loads node state, metrics, and activity entries from existing mobile data helpers
- route `/control/mobile/nodes/{id}/summary` through the migrated report composition without changing the shipped JSON contract
