# Phase 58: Greenfield Core Shell and Service Interfaces - Context

**Gathered:** 2026-03-28
**Status:** Ready for execution

## Goal

Introduce a clean core shell and stable service interfaces so future work can land in the new lane instead of attaching directly to legacy command and runtime modules.

## What We Know

- Phase 57 chose setup handoff reporting as the first proving slice because it already spans onboarding state, inspection, one control route, and one Control UI panel.
- The current implementation keeps the report-building logic in `crates/cli/src/commands/inspect.rs`, while the source state and helper functions still live in `onboard.rs`.
- `openrustclaw-core` already provides the shared result and error surface that a new application crate can build on.
- A new shell should not depend on clap, axum route registration, or HTML rendering details.

## Constraints

- This phase should create the new boundary without changing shipped runtime behavior yet.
- The first application service must be generic enough for later adapters, but specific enough to prove the slice can move out of CLI command modules.
- The new shell must compile cleanly inside the existing workspace and not increase coupling back into legacy command hubs.

## Implementation Direction

- add a new `openrustclaw-app` crate as the first greenfield application shell
- define the setup-handoff state, report, and source trait there, using `openrustclaw-core` for the shared result surface
- cover the service with focused unit tests so Phase 59 can wire adapters into it safely
