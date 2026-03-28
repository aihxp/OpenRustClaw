# Phase 60: Brownfield Containment and Contributor Defaults - Context

**Gathered:** 2026-03-28
**Status:** Ready for execution

## Goal

Make the new lane the default for future work through contributor guidance, compatibility rules, and explicit deprecation follow-up.

## What We Know

- Phase 57 defined the transition contract and ranked migration inventory.
- Phase 58 added `openrustclaw-app` as the first greenfield application shell.
- Phase 59 proved the lane with a real migrated setup handoff slice.
- Contributor-facing guidance already mentions the greenfield transition, but it still needs stronger defaults now that the first shipped slice is real.

## Constraints

- This phase should tighten guidance without claiming legacy hotspots are already fully migrated.
- The repo still needs compatibility work in large command modules, so the rules must allow bounded fixes while blocking new cross-cutting growth there by default.
- Follow-up migration debt needs to stay visible instead of being implied away by the new architecture language.

## Implementation Direction

- update contributor and agent-facing docs so `openrustclaw-app` is the default home for new application logic
- make compatibility-only exceptions explicit for legacy command hubs
- preserve the next migration queue in planning-facing cleanup and greenfield contracts
