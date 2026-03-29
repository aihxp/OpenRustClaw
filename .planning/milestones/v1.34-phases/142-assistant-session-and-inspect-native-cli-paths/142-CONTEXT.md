# Phase 142: Assistant, Session, and Inspect Native CLI Paths - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Define the first assistant, session, and inspect native CLI operator-path slice so the successor CLI path reaches real user-facing command families.

</domain>

<decisions>
## Implementation Decisions

- Treat assistant, session, and inspect as the first bounded operator-path slice instead of trying to replace every CLI family at once.
- Ground the slice in the real operator-facing hotspots under `crates/cli/src/commands/assistant.rs`, `session.rs`, `chat.rs`, and `inspect.rs`.
- Keep the phase at the planning-contract level so later source work can implement one real operator-path handoff without rediscovering command-family ownership.

</decisions>
