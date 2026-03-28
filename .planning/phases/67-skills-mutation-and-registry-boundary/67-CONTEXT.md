# Phase 67: Skills Mutation and Registry Boundary - Context

**Gathered:** 2026-03-28
**Status:** Ready for execution

## Goal

Start the next truthful `skills.rs` migration by moving one mutation-heavy or registry-heavy lane behind a real service seam.

## What We Know

- `crates/cli/src/commands/skills.rs` still owns the install, update, and uninstall orchestration inline, including registry calls, policy checks, DB writes, event publication, compile attempts, and result shaping.
- That mutation lane already feeds the shipped control API through `install_data(...)`, `update_data(...)`, and `uninstall_data(...)`, so it is a real product surface rather than a CLI-only helper.
- The compiled-skill overview seam from Phase 64 already proved a read-only extraction path; this phase needs to move a write-heavy path instead of reopening the overview service.
- The greenfield win here is to move the mutation orchestration and result composition into `openrustclaw-app` while keeping `skills.rs` as the adapter around workspace, DB, registry, and compile primitives.

## Constraints

- This phase must extract a real mutation-heavy lane, not just rename helpers inside `skills.rs`.
- The existing install, update, and uninstall mutation contract returned to the runtime or control API must stay stable.
- The migration should preserve a truthful follow-up queue for the remaining plugin-binding and voice-plugin mutation paths still owned by `skills.rs`.

## Implementation Direction

- add a skill registry mutation service to `openrustclaw-app`
- move install, update, and uninstall orchestration plus result shaping behind that service
- keep `skills.rs` as the async adapter that loads config, talks to the registry, persists DB state, publishes events, and compiles artifacts
