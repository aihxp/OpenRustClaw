# Phase 141: Native CLI Dispatch Bootstrap - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Define the first source-level native CLI dispatch slice so the implementation roadmap has an explicit successor routing path for top-level `openrustclaw` ownership instead of a general future CLI replacement claim.

</domain>

<decisions>
## Implementation Decisions

- Treat the first native CLI dispatch slice as explicit implementation-contract data instead of leaving successor routing ownership as a broad future idea.
- Tie the slice directly to `crates/cli/src/main.rs` and `crates/cli/src/commands/mod.rs` so the top-level hotspot is grounded in the real command surface.
- Keep the phase at the planning-contract level so later source work can build one concrete CLI dispatch handoff slice first.

</decisions>
