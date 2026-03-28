# Phase 85: Autonomy Lessons and Lesson Mutation Route Families - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning

## Phase Boundary

Move the autonomy lesson listing and lesson mutation route families out of `start.rs` so lesson-control flows stop owning registry shaping and mutation-result composition inside the control-plane route hub.

## Decisions

- The extraction should cover the lesson-oriented autonomy surfaces first, including the lesson list, create, and deactivate flows, while keeping broader autonomy and orchestration follow-on work out of scope.
- `start.rs` should remain the HTTP adapter for the lesson routes and status-code mapping.
- The new application service should own stable lesson summary shaping and post-mutation report composition.

## Existing Code Insights

- `start.rs` still owns `/control/autonomy`, `/control/autonomy/lessons`, lesson creation, and lesson deactivation orchestration inline.
- The current handlers duplicate the same registry-description shaping after every lesson mutation.
- The seam is bounded because lesson-control uses the existing `control` module for persistence and only needs one stable application-owned service boundary above it.
