# Phase 72: High-Value Control Route Follow-On Extraction - Context

**Gathered:** 2026-03-28
**Status:** Ready for execution

## Goal

Move one more bounded `/control/...` route family behind the new greenfield services introduced by this milestone so the remaining runtime route coupling keeps shrinking.

## What We Know

- `crates/cli/src/commands/start.rs` still hand-assembles the `/control/runtime/vault` route family inline through three handlers: status, set, and delete.
- That route family now sits directly on top of the new runtime vault mutation seam from Phase 70, which makes it the cleanest truthful follow-on extraction in this milestone.
- The greenfield win here is to move the route-family behavior into `openrustclaw-app` while keeping `start.rs` as the HTTP adapter around request parsing and response mapping.

## Constraints

- This phase must extract one real bounded control route family, not just rename handlers inside `start.rs`.
- The shipped `/control/runtime/vault` API contract must stay stable for the runtime API surface.
- The extraction should materially reduce route-local cross-calls around the newly migrated runtime vault seam.

## Implementation Direction

- add a runtime vault control service to `openrustclaw-app`
- move route-family behavior and response shaping behind that service
- keep `start.rs` as the HTTP adapter that converts requests into the runtime vault control service
