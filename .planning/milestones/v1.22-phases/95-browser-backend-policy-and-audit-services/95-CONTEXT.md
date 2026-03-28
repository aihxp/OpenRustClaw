# Phase 95: Browser Backend Policy and Audit Services - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Move browser backend policy and audit handling behind `openrustclaw-app` so browser control decisions stop deepening legacy command ownership.

</domain>

<decisions>
## Implementation Decisions

- Keep audit-log file reads and appends in `crates/cli/src/commands/browser.rs`.
- Move backend policy normalization, denial decisions, and audit-entry shaping into an app-side browser backend control service.
- Preserve the existing browser backend policy and audit payloads while reducing command-local policy logic.

</decisions>
