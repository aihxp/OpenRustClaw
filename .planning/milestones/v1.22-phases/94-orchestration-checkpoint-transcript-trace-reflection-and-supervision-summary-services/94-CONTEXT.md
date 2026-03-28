# Phase 94: Orchestration Checkpoint, Transcript, Trace, Reflection, and Supervision Summary Services - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Move the remaining orchestration checkpoint, transcript, trace, reflection, and supervision summary composition behind `openrustclaw-app` so `orchestrate.rs` keeps shrinking toward an adapter-only surface.

</domain>

<decisions>
## Implementation Decisions

- Keep receipt and active-run file reads in `crates/cli/src/commands/orchestrate.rs`.
- Move report composition, resource aggregation, reflection-candidate generation, and attention-signal shaping into a dedicated app-side reporting service.
- Preserve the existing orchestration inspection payloads while reducing read-path business logic in the legacy command module.

</decisions>
