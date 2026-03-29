# Phase 143: Control and Runtime Native CLI Handoff - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Define the first bounded control and runtime native CLI handoff so the biggest remaining operator command hotspots have an explicit successor path.

</domain>

<decisions>
## Implementation Decisions

- Treat the first control and runtime handoff as explicit bounded ownership transfer instead of broad future CLI cleanup.
- Ground the handoff directly in `crates/cli/src/commands/control.rs`, `runtime.rs`, and `crates/cli/src/main.rs`.
- Keep the phase at the planning-contract level so later source work can implement one concrete control or runtime handoff slice without rediscovering hotspot ownership.

</decisions>
