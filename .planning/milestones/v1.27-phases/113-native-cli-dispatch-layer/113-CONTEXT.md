# Phase 113: Native CLI Dispatch Layer - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Define the native top-level CLI dispatch layer so `main.rs` stops being the permanent routing owner for the product path.

</domain>

<decisions>
## Implementation Decisions

- Treat CLI dispatch as its own delivery-layer concern instead of extending `main.rs` with another generation of routing helpers.
- Route native CLI dispatch through app ports instead of legacy command-to-command orchestration.
- Preserve the `openrustclaw` binary contract while shrinking `main.rs` toward thin bootstrap ownership.

</decisions>
