# Phase 99: Secondary Operator, Media, Tools, and Memory Service Seams - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Move the targeted residual operator, media, tools, and memory helper seams behind `openrustclaw-app` so neighboring secondary command modules stop deepening brownfield ownership.

</domain>

<decisions>
## Implementation Decisions

- Keep artifact I/O, runtime calls, workspace reads, and transport wiring in the CLI command modules.
- Move tool-host profiling helpers, media prompt and response shaping, and memory render or parse helpers into dedicated `openrustclaw-app` services.
- Preserve the current operator-facing helper contracts while reducing helper-heavy business logic in the legacy command modules.

</decisions>
