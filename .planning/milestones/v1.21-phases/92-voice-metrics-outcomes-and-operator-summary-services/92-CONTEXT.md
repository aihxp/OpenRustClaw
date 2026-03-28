# Phase 92: Voice Metrics, Outcomes, and Operator Summary Services - Context

**Gathered:** 2026-03-28
**Status:** Completed
**Mode:** Autonomous

<domain>
## Phase Boundary

Move the voice metrics, outcomes, and operator-summary composition helpers out of `voice_runtime.rs` so those voice operator surfaces stop depending on command-local summary composition.

</domain>

<decisions>
## Implementation Decisions

- Keep artifact metadata probing and session-file loading in `voice_runtime.rs`.
- Move transcript, event, artifact, metrics, and outcome composition into `openrustclaw-app`.
- Preserve the shipped operator-facing voice reporting shapes.

</decisions>
