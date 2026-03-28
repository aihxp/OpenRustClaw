# Phase 82: Diagnostics and Live Operator Event Route Family - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning

## Phase Boundary

Move the remaining diagnostics summary and live diagnostics websocket or session orchestration out of `start.rs` so operator diagnostics stop deepening the largest route hub.

## Decisions

- The websocket transport loop can remain in `start.rs`, but diagnostics collection, interval normalization, and event-payload shaping should move behind the application service lane.
- The route family should preserve the current diagnostics JSON contract and continue returning the same error semantics.
- The extraction should not reopen the broader doctor command implementation; it should adapt that existing report source into a stable application seam.

## Existing Code Insights

- `control_diagnostics_handler` still calls `doctor::collect_report` inline from `start.rs`.
- `diagnostics_ws_session` still owns interval normalization and the `"diagnostics"` versus `"error"` websocket payload shaping.
- The seam is narrow enough to extract without changing the underlying diagnostics checks or onboarding repair logic.
