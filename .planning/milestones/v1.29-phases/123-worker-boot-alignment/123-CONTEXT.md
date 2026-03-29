# Phase 123: Worker Boot Alignment - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Align mobile, voice, and orchestration worker boot contracts with the native runtime-host delivery path instead of preserving separate legacy startup assumptions.

</domain>

<decisions>
## Implementation Decisions

- Group the major worker families into one planning slice because they all need to target the same runtime-host delivery model.
- Define compatibility shims explicitly so worker-family command entrypoints can remain bounded while native worker boot becomes the real owner.
- Keep worker boot alignment focused on delivery ownership, not repository-adapter implementation, so the milestone stays incremental.

</decisions>
