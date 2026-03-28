# Phase 86: Remaining Skill-Control Route Families - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning

## Phase Boundary

Move the remaining non-voice-call skill-control route families out of `start.rs` so compile, compiled-artifact, background-service, invoke, execute, and adjacent skill-control flows stop owning route-local orchestration.

## Decisions

- Keep the voice-call and channel-extension routes out of scope for this phase; they remain the dedicated Phase 87 seam.
- Preserve `start.rs` as the HTTP adapter for status codes, request parsing, and operator-result recording.
- Let one shared application service own the skill-control orchestration boundary above the existing `skills` command helpers.

## Existing Code Insights

- `start.rs` already had the new `SkillControlService` shell and source adapter, but the phase still needed a stable request model, deterministic route verification, and a completed planning record.
- The remaining in-scope handlers all adapt through the same legacy `skills` helper surface, which makes them a coherent bounded service-lane extraction.
- The safest verification path is a workspace-local skill fixture exercised through the runtime router so compile, inspection, background-service, invoke, and execute flows can be proven without network dependencies.
