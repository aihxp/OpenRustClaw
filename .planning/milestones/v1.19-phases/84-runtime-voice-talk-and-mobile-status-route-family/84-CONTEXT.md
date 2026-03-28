# Phase 84: Runtime, Voice, Talk, and Mobile Status Route Family - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning

## Phase Boundary

Move the remaining read-heavy runtime, voice, talk, and mobile status route family out of `start.rs` so status report composition no longer lives inline inside the control-plane route hub.

## Decisions

- The extraction should focus on read-heavy status surfaces only, leaving mutations and deeper detail flows out of scope.
- `start.rs` should remain the HTTP adapter for query parsing, response mapping, and route wiring.
- The new application service should own limit defaulting, mobile node result shaping, and stable route-family report composition.

## Existing Code Insights

- `start.rs` still owns runtime status, operator ops, voice status, voice providers, voice metrics, voice operator reports, voice outcomes, voice sessions, voice session health, talk status, talk metrics, talk sessions, mobile nodes, and mobile node summary orchestration inline.
- The route family is mostly read-only and already shares stable response patterns, which makes it a good compatibility-preserving extraction seam.
- The seam is well bounded because the shipped status surfaces can move behind one app service without reopening unrelated control mutations or setup flows.
