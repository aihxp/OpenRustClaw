# Full Greenfield Conversion Roadmap

**Created:** 2026-03-28
**Purpose:** Canonical follow-on roadmap for pushing OpenRustClaw from the retired `18/18` seam ledger toward a fully greenfield architecture where legacy command modules are adapter-only surfaces.
**Status:** Completed 2026-03-28 at `6/6` shipped milestones, or `100%`

## What "Full Greenfield Conversion" Means

OpenRustClaw should treat full conversion as an architecture outcome, not a line-count target.

The repo is fully greenfield-converted when:

- application or domain-owned services hold product and operator business logic
- legacy command modules are reduced to adapters for CLI parsing, HTTP transport, workspace I/O, and compatibility glue
- new features default to `openrustclaw-app` or successor application crates instead of landing in legacy hotspots
- verification targets application services directly for most new behavior instead of requiring route-local or command-local orchestration tests

## Measurement Model

| Dimension | Target State | Failure Signal |
| --- | --- | --- |
| Logic ownership | business rules live in application or domain layers | route handlers or command modules still decide policy, mutation flow, or report composition |
| Transport isolation | CLI and HTTP modules only parse input, call services, and shape output | handler-local orchestration grows inside `start.rs`, `mobile.rs`, `voice_runtime.rs`, or peers |
| Persistence boundaries | durable storage and external integrations hang off explicit adapters or ports | application rules depend directly on mixed command-module helpers |
| Test topology | most new tests hit application services directly | regression safety depends on large end-to-end command tests for basic business logic |
| Contribution defaults | contributors treat legacy hotspots as compatibility-only unless a migration phase says otherwise | new cross-cutting behavior lands in oversized command files by default |

## Program Milestones

### v1.19 Full Greenfield Conversion: Control Plane Route Families I

Primary target: the highest-friction remaining `start.rs` route families.

- `/control/config`, `/control/config/validate`, and `/control/config/update`
- diagnostics summary and live diagnostics websocket or session flows
- channel registry account and binding lifecycle flows
- read-heavy runtime, voice, talk, and mobile status handlers

### v1.20 Full Greenfield Conversion: Control Plane Route Families II

Primary target: remaining control-plane route families that still compose business logic inside `start.rs`.

- autonomy lessons and lesson mutation surfaces
- remaining skill-control route families that are still route-local adapters over command-local logic
- voice-call and channel-extension control route families that still depend on legacy route orchestration
- route registration and shared control state cleanup after the first two route milestones

### v1.21 Full Greenfield Conversion: Mobile and Voice Runtime Services

Primary target: the largest remaining operator runtime command hubs after `start.rs`.

- mobile notification, outbound message, dispatch, approval, wake, and rehydrate lifecycle lanes
- mobile sync, push, and runtime state aggregation helpers still owned by `mobile.rs`
- voice provider resolution, session lifecycle, and state mutation lanes in `voice_runtime.rs`
- voice metrics, outcomes, and operator-summary composition in `voice_runtime.rs`

### v1.22 Full Greenfield Conversion: Orchestration and Browser Services

Primary target: the deepest remaining agentic and browser execution hubs.

- orchestration request routing, override validation, and lifecycle-state transitions
- checkpoint, transcript, trace, reflection, and supervision summarization
- browser backend policy and audit handling
- browser session persistence, workflow execution, inspection, and sequence orchestration

### v1.23 Full Greenfield Conversion: Setup and Secondary Command Surfaces

Primary target: remaining mixed-responsibility command surfaces outside the major hotspots.

- onboarding, repair, and resume orchestration that still lives in `onboard.rs`
- residual lifecycle seams in `channels.rs`, `schedule.rs`, `services.rs`, and `control.rs`
- residual operator or media helpers in `tools.rs`, `media.rs`, `memory.rs`, and neighboring command modules
- cleanup of helper duplication created during the brownfield-to-greenfield transition

### v1.24 Full Greenfield Conversion: Adapter-Only Exit and Enforcement

Primary target: the architectural end-state and the rules that keep it true.

- final residual helper extraction or deletion in legacy hotspots
- explicit adapter or port boundaries for remaining persistence and external side effects
- architecture guardrails that block new business logic from landing in legacy command hubs
- final full-conversion audit, verification bundle, and exit scorecard

## Sequence Rationale

The order is deliberate:

1. shrink `start.rs` first because it is the largest shared route hotspot and the default place future logic drifts into
2. shrink `mobile.rs` and `voice_runtime.rs` next because they are the next biggest operator-owned command surfaces
3. move `orchestrate.rs` and `browser.rs` after that because they combine larger behavior breadth with deeper testing cost
4. finish with setup and smaller command surfaces once the highest-risk architectural defaults are already reversed
5. close with enforcement, audit, and adapter-only exit criteria so the repo does not regress after the extraction work lands

## Exit Criteria

OpenRustClaw should only claim full greenfield conversion when all of the following are true:

- `start.rs`, `mobile.rs`, `voice_runtime.rs`, `orchestrate.rs`, `browser.rs`, `skills.rs`, `runtime.rs`, `inspect.rs`, and `onboard.rs` act primarily as adapters
- the dominant business rules for operator-facing behavior live in application or domain-owned services
- new route families and command surfaces default to application-owned orchestration
- the architecture rules are enforced by tests, review defaults, or CI checks instead of relying on contributor memory alone
- a canonical exit audit says the remaining legacy command surfaces are compatibility wrappers rather than hidden ownership hubs

## Enforcement Defaults

After `v1.24`, contributors should treat the adapter-only contract as a standing repo rule:

- new business rules, report composition, and operator-facing policy logic default to `openrustclaw-app` or a successor application crate
- legacy command modules may parse transport input, load workspace or persistence state, call application services, and shape CLI or HTTP output
- legacy command modules should not grow new cross-cutting helper families when those helpers decide validation, mutation, summary, or orchestration behavior
- any exception should be justified explicitly in milestone planning instead of being treated as the new default
- regression tests for new behavior should prefer application-service coverage first, with adapter tests reserved for transport or persistence wiring

The migrated end-state now depends on named adapter seams instead of helper sprawl in the final hotspots:

- `inspect.rs` bridges assistant continuity and tool execution history through `AssistantContinuityService`, `ToolExecutionAuditService`, and `ToolExecutionAuditFileStore`
- `skills.rs` bridges compiled-skill reference reads and voice-call reporting through `CompiledSkillMcpService` and `VoiceCallReportingService`
- `start.rs` bridges compiled-skill MCP registration through `CompiledSkillWorkspaceCatalog` over `CompiledSkillMcpService`

## Companion Documents

- `.planning/codebase/GREENFIELD.md` — original greenfield transition contract and containment rules
- `.planning/codebase/GREENFIELD-INVENTORY.md` — retired historical `18/18` seam ledger from `v1.13` through `v1.18`
- `.planning/ROADMAP.md` — active milestone phases
- `.planning/PROJECT.md` — project-level milestone context and decisions
