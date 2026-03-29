# Phase 116: CLI Boundary Separation and Compatibility Shim Plan - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Make parsing, rendering, app invocation boundaries, and the temporary compatibility shim plan explicit enough to implement the first native CLI slice safely.

</domain>

<decisions>
## Implementation Decisions

- Treat parse, render, and app invocation as separate responsibilities so native CLI delivery does not recreate mixed command helpers.
- Allow temporary compatibility shims only after native delivery ownership exists and only as forwarding shells.
- Make the shim boundary part of the milestone exit criteria so the `3/8` progress claim stays truthful.

</decisions>
