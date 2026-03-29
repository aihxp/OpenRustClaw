# Phase 101: Final Residual Helper Extraction and Hotspot Deletion - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Extract or delete the last targeted helper-owned seams still hiding business logic inside the remaining legacy command hotspots.

</domain>

<decisions>
## Implementation Decisions

- Focus the final extraction slice on real residual ownership in `inspect.rs`, `skills.rs`, and `start.rs` instead of opening another broad rewrite queue.
- Move continuity, tool-audit, voice-call reporting, and compiled-skill MCP behavior into `openrustclaw-app` so the command modules lose helper-owned business rules.
- Accept only compatibility-preserving adapter changes in the command modules while the new services absorb the logic.

</decisions>
