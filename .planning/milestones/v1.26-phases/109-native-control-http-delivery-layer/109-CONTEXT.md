# Phase 109: Native Control HTTP Delivery Layer - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Define the native control HTTP delivery layer so future control-route work stops defaulting to `crates/cli/src/commands/start.rs`.

</domain>

<decisions>
## Implementation Decisions

- Treat control HTTP delivery as a gateway ownership problem, not as another extension of the CLI command tree.
- Use `ControlPlanePort` in `openrustclaw-app` as the business-use contract instead of command-module cross-calls.
- Preserve compatibility by allowing bounded forwarding only after the native gateway route ownership is explicit.

</decisions>
