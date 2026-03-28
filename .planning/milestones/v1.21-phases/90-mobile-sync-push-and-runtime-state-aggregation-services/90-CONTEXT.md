# Phase 90: Mobile Sync, Push, and Runtime State Aggregation Services - Context

**Gathered:** 2026-03-28
**Status:** Completed
**Mode:** Autonomous

<domain>
## Phase Boundary

Move the mobile sync, push, and runtime state aggregation helpers out of `mobile.rs` so those mobile runtime summaries stop depending on command-local report composition.

</domain>

<decisions>
## Implementation Decisions

- Keep runtime-state persistence and app-session recording in `mobile.rs`.
- Move heartbeat, push, sync, runtime-status refresh, activity shaping, and summary rollups into `openrustclaw-app`.
- Preserve the current operator-facing response shapes.

</decisions>
