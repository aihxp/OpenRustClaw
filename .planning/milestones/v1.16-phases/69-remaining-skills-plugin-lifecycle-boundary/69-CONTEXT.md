# Phase 69: Remaining Skills Plugin Lifecycle Boundary - Context

**Gathered:** 2026-03-28
**Status:** Ready for execution

## Goal

Move the next real plugin-lifecycle mutation lane out of `skills.rs` so the hotspot keeps shrinking after the first registry mutation extraction.

## What We Know

- `crates/cli/src/commands/skills.rs` still owns the voice-plugin binding lane inline, including compiled-skill validation, declared-plugin checks, binding record shaping, registry mutation, and plugin-event publication.
- That lane already feeds the shipped control API through `/control/skills/voice-plugins/bind`, so it is a real product surface rather than a CLI-only helper.
- The greenfield win here is to move the binding business rules and result shaping into `openrustclaw-app` while keeping `skills.rs` as the adapter around compiled artifact resolution, registry persistence, and event publication.

## Constraints

- This phase must migrate one real remaining plugin-binding lane, not just rename helpers inside `skills.rs`.
- The shipped voice-plugin bind contract must stay stable for both CLI and control API callers.
- The migration should leave a clear follow-on queue for the remaining voice-plugin and auth-plugin lifecycle work still owned by `skills.rs`.

## Implementation Direction

- add a voice-plugin binding service to `openrustclaw-app`
- move binding validation and binding record composition behind that service
- keep `skills.rs` as the adapter that resolves compiled-skill details, persists the registry entry, and publishes the plugin lifecycle event
