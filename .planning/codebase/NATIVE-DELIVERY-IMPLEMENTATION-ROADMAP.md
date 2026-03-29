# Native Delivery Implementation Roadmap

**Created:** 2026-03-28
**Purpose:** Canonical follow-on roadmap for turning the completed native-delivery planning scorecard into source-level native entrypoints, repository adapters, and retired legacy delivery ownership.
**Status:** Active after `v1.33` at `1/6` shipped milestones, or about `17%`, with `v1.34` targeting `2/6`, or about `33%`
**Baselines preserved:** historical greenfield seam ledger closed at `18/18`; adapter-only full-conversion roadmap closed at `6/6`; native-delivery planning roadmap closed at `8/8`

## What "Continue Greenfield Conversion" Means Now

OpenRustClaw has already closed the planning denominators for seam migration, adapter-only ownership, and native-delivery design. The next truthful queue is implementation, not more planning-only completion claims.

This roadmap measures:

- source-level native entrypoint replacement, not just architectural intent
- verified retirement of legacy delivery ownership, not just bounded future plans
- repository and integration adapter lift into explicit native infrastructure contracts
- packaging, docs, and verification that reflect what is implemented in code

## Milestone Sequence

### v1.33 Native Delivery Implementation: Gateway and MCP Successor Entry Points

Status after shipment: complete. This milestone defined the first concrete source-level successor-entry implementation slice by naming the gateway-native control bootstrap, the MCP-native startup path, the Control UI serving handoff, and the first bounded `start.rs` compatibility-and-verification rules needed to implement that slice without rediscovering startup ownership.

- gateway-native control bootstrap ownership over the first successor startup path
- MCP-native startup ownership plus Control UI serving alignment to the gateway path
- first bounded startup responsibilities leaving `start.rs`
- direct successor-entry verification plus compatibility rules for the initial handoff slice

## v1.33 Outcome

`v1.33` did not implement the gateway and MCP successor entrypoints in source yet. It advanced the implementation roadmap to `1/6` by defining the first source-level handoff slice explicitly enough to build and verify instead of leaving gateway, MCP, and Control UI startup under one large legacy bootstrap hotspot.

- the implementation roadmap now names the first implemented gateway-native control bootstrap slice explicitly
- the implementation roadmap now names the first implemented MCP-native startup slice and the Control UI serving handoff tied to it
- `start.rs` no longer survives as a vague first-step bootstrap owner in the roadmap; the first bounded responsibilities leaving it are explicit
- the roadmap now defines how the first successor entrypoints are verified directly while compatibility forwarding remains bounded

### v1.34 Native Delivery Implementation: Native CLI Dispatch and Core Operator Paths

Primary target: move the top-level CLI dispatch and first operator entrypoints off the legacy command tree.

- native CLI dispatch layer in `openrustclaw-cli`
- assistant, chat, session, inspect, control, and runtime entrypoint lift
- command-to-command routing removal for the first operator families
- compatibility shims only where required to preserve the shipped interface

### v1.35 Native Delivery Implementation: Runtime Hosts and Background Workers

Primary target: implement native runtime-host and worker boot entrypoints so worker lifecycle startup no longer depends on legacy command bootstraps.

- native runtime-host bootstrap for service-manager, probes, maintenance, and scheduler flows
- mobile, voice, and orchestration worker boot migration
- legacy startup ownership removal from `start.rs`, `runtime.rs`, and adjacent worker helpers
- focused runtime-host verification instead of command-local startup tests

### v1.36 Native Delivery Implementation: Repository and Integration Adapter Lift

Primary target: replace command-local persistence and side-effect helpers with explicit repositories and infrastructure adapters used by app ports.

- repository adapters for sqlite, workspace files, audit logs, runtime config, compiled-skill cache, and registries
- infrastructure gateways for providers, channels, and external service integrations
- app-service migration from command-local helpers to repository or adapter contracts
- direct repository and integration verification over native surfaces

### v1.37 Native Delivery Implementation: Legacy Command Tree Retirement

Primary target: delete, isolate, or freeze the superseded command-tree hotspots once successor paths are implemented.

- retire or remove `crates/cli/src/commands/*` families that leave the main product path
- shrink `main.rs` to thin bootstrap-only ownership or replace it outright
- preserve only bounded native shims where deletion is not yet safe
- enforce source-level guardrails so retired files cannot regain ownership

### v1.38 Native Product Verification and Packaging Exit

Primary target: verify the source-level native-product claim end to end and align packaging, docs, and compatibility statements to that implemented state.

- end-to-end verification of the native-delivery scorecard against shipped code
- final packaging and docs alignment to implemented native entrypoints
- explicit audit of any remaining native shims or true exceptions
- final source-level native-product exit claim

## Sequence Rationale

The order is deliberate:

1. implement gateway and MCP successor entrypoints first because `start.rs` is the largest remaining shared bootstrap hotspot
2. move top-level CLI dispatch next so user-facing operator paths stop flowing through the legacy command tree
3. move runtime hosts after the core delivery surfaces exist so worker startup can point at real native entrypoints
4. lift repositories and integration adapters after native callers exist so infrastructure contracts serve implemented product paths
5. delete or isolate the legacy command tree only after successor entrypoints and adapters are running
6. close with source-level verification and packaging truth so the final claim is evidence-backed

## Exit Criteria

OpenRustClaw should only claim source-level native delivery completion when all of the following are true:

- control HTTP, Control UI, MCP, CLI, and runtime-host entrypoints run through implemented native delivery surfaces
- `crates/cli/src/commands/*` is deleted, frozen, or reduced to clearly temporary native shims outside the primary product path
- app ports call repositories and infrastructure adapters instead of command-local helpers
- tests and CI verify native entrypoints directly
- docs and packaging describe the implemented native path instead of the retired command-tree story
- any remaining exception is explicit, bounded, and justified by a published audit

## Companion Documents

- `.planning/codebase/GREENFIELD-INVENTORY.md` — retired historical `18/18` seam ledger
- `.planning/codebase/GREENFIELD-FULL-CONVERSION.md` — completed adapter-only roadmap
- `.planning/codebase/NATIVE-DELIVERY-LAYER-ROADMAP.md` — completed native-delivery planning roadmap
- `.planning/ROADMAP.md` — active milestone phases
- `.planning/PROJECT.md` — project-level milestone context and decisions
