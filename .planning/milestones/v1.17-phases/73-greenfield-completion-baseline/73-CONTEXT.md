# Phase 73: Greenfield Completion Baseline - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning

## Phase Boundary

Define one canonical ranked seam inventory for the brownfield-to-greenfield transition and derive a truthful completion percentage from that inventory so future progress updates stop relying on vague milestone counts.

## Decisions

- The denominator should count ranked seam migrations, not phase count, file count, or line count.
- The proving-slice bootstrap work should stay outside the denominator so the percentage reflects the follow-on migration queue instead of milestone scaffolding.
- The baseline should exist in both planning form and a small reusable application-layer report so later shipped surfaces can consume it directly.

## Existing Code Insights

- `openrustclaw-app` currently exposes the migrated seams from phases 61-72 but there is no single progress-report service.
- `.planning/codebase/CLEANUP.md` still calls out `start.rs` and `skills.rs` as structural hotspots, which matches the remaining queue.
- `skills.rs`, `runtime.rs`, and `start.rs` remain the dominant legacy hubs for the next follow-on seams.

## Specific Ideas

- add a `greenfield_progress` report service to `openrustclaw-app`
- add one canonical seam inventory doc under `.planning/codebase/`
- derive the percentage from the ranked inventory and keep the denominator explicit
