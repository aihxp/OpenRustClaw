# Phase 145: Native Runtime-Host Bootstrap - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Define the first native runtime-host bootstrap slice so service-manager, probes, maintenance, and scheduler startup no longer remain a broad future replacement claim.

</domain>

<decisions>
## Implementation Decisions

- Treat the first native runtime-host bootstrap slice as explicit implementation-contract data instead of leaving successor startup ownership as a broad future idea.
- Tie the slice directly to the real startup hotspots in `crates/cli/src/main.rs`, `start.rs`, and `runtime.rs` so the runtime-host handoff is grounded in the current codebase.
- Keep the phase at the planning-contract level so later source work can build one concrete runtime-host handoff slice first.

</decisions>
