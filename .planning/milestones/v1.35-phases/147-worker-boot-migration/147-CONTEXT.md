# Phase 147: Worker Boot Migration - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Define the mobile, voice, and orchestration worker-boot migration slice over native runtime-host ownership.

</domain>

<decisions>
## Implementation Decisions

- Group the major worker families into one implementation slice because they all need to target the same runtime-host successor model.
- Define compatibility shims explicitly so worker-family command entrypoints can remain bounded while native worker boot becomes the real owner.
- Keep worker-boot migration focused on delivery ownership, not repository-adapter implementation, so the milestone stays incremental.

</decisions>
