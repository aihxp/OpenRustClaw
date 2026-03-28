# Phase 87: Voice-Call and Channel-Extension Control Route Families - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning

## Phase Boundary

Move the remaining voice-call lifecycle and channel-extension control route families out of `start.rs` so those runtime control surfaces stop owning route-local business orchestration.

## Decisions

- Keep the already-extracted voice-plugin and non-voice-call skill-control surfaces out of scope; this phase only handles the still-direct voice-call and channel-extension seams.
- Preserve `start.rs` as the HTTP adapter and operator-result recorder for voice-call start, reconnect, and end flows.
- Use one bounded application service above the existing `skills` helpers so the final direct route-local voice/channel orchestration leaves the route hub.

## Existing Code Insights

- `start.rs` still directly owned voice-call list, health, metrics, event, artifact, start, reap, reconnect, end, channel-extension list, and bind flows.
- Those handlers all depended on the same `skills` helper lane and already shared the same error/status mapping shape, which made them a coherent extraction seam.
- The safest router-level verification is a local workspace skill that can bind both a voice plugin and a channel extension without external network or provider dependencies.
