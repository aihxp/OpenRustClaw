# Phase 6: Deployment, Runtime, and Operator Ops - Context

**Gathered:** 2026-03-26
**Status:** Ready for planning

<domain>
## Phase Boundary

Make the shipped runtime operational path production-credible. This phase hardens the existing deployment, managed-service, health, reload, backup, upgrade, rollback, and diagnostics surfaces so operators can deploy OpenRustClaw, keep it running, and recover it without relying on tribal knowledge.

</domain>

<decisions>
## Implementation Decisions

### MVP operator-ops boundary
- **D-01:** Phase 6 should consolidate and document the runtime operations surfaces already present in the Rust CLI and control plane instead of inventing a new deployment stack.
- **D-02:** The MVP deployment lane is the shipped workspace-owned runtime path: config, `openrustclaw start`, managed user service install, health and beacon inspection, bounded backup and restore, and upgrade or rollback planning.
- **D-03:** Success means operators can answer three practical questions quickly: is the runtime healthy, how is it meant to be restarted, and what is the safest recovery path if a rollout goes bad.

### Recovery and diagnostics
- **D-04:** Runtime recovery should begin from one operator-visible summary that pulls together health, service-install state, runtime lock state, and restart or recovery guidance before asking operators to inspect raw JSON or workspace files.
- **D-05:** Existing `.claw/control/` and `.claw/runtime-*` artifacts remain the durable source of truth for operations; Phase 6 should reuse them rather than introduce a second persistence model.
- **D-06:** Upgrade, self-update, rollback, backup, and restore are already bounded CLI paths; this phase should make those paths easier to discover and trust from shipped operator surfaces.

### Documentation and verification
- **D-07:** Deployment docs must align with the actual shipped runtime commands and host-service behavior instead of older generic deployment prose.
- **D-08:** Verification should prove the operator-ops contract across multiple surfaces, not just individual helper functions in isolation.

### the agent's Discretion
- The exact mix between control-surface work, CLI-facing runtime summaries, and docs can move between plans as long as the phase ends with one coherent deploy-run-recover story.
- It is acceptable to favor the local or single-node production path before deeper cluster-specific guidance if that yields a more truthful MVP operator contract.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Runtime operations and recovery
- `crates/cli/src/commands/runtime.rs` — runtime status, health, beacon, reload plan, service install, backup, restore, upgrade, self-update, rollback, and lock inspection
- `crates/cli/src/main.rs` — shipped CLI entry points for `openrustclaw runtime ...` and `openrustclaw runtime services ...`
- `crates/cli/src/commands/start.rs` — control-plane runtime endpoints and control UI route wiring
- `crates/cli/src/commands/services.rs` — runtime service, scheduler, readiness, and log/event diagnostics

### Browser operator surface
- `crates/cli/src/commands/control_ui.html` — shipped browser dashboard for runtime and service inspection
- `crates/cli/src/commands/control_ui.rs` — dashboard surface regression tests

### Documentation and verification anchors
- `README.md` — top-level operator guidance and runtime claims
- `docs/src/getting-started/installation.md` — first production-capable install path
- `docs/src/deployment/production.md` — canonical deployment and recovery guide
- `tests/integration/src` — cross-surface runtime and operator-trust verification

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `runtime.rs` already ships meaningful operator primitives: managed user-service detection and install, runtime health scans, liveness beacons, reload plans, config migration, runtime lock inspection, workspace backup and restore, and bounded upgrade or rollback planning.
- `start.rs` already exposes control endpoints for runtime health, beacon, reload plan, upgrade plan, self-update plan, rollback plan, and service-level diagnostics.
- `control_ui.html` already displays health, beacon, reload plan, channel readiness, logs, and runtime events, so this phase can extend an existing operator shell rather than add a new UI surface.
- `production.md` already mentions several runtime commands, which gives a base to tighten rather than a blank page to author from scratch.

### Established Patterns
- Earlier phases improved operator trust by surfacing typed summaries before raw artifacts and then aligning README and quickstart around the same inspection loop.
- Integration coverage has been used to prove cross-surface trust contracts, especially when one feature spans persisted artifacts, control APIs, and UI exposure.
- Phase 5 established a good documentation pattern for operations-adjacent work: readiness first, recent summarized outcomes second, deeper artifact inspection only when needed.

### Gaps to Close
- Runtime operations are still fragmented across several control endpoints and CLI commands; there is no single shipped operator summary for deploy-run-recover decisions.
- Control UI exposes runtime health primitives but does not yet foreground managed-service install state, lock state, or the recovery path alongside them.
- Production docs still mix older broad deployment guidance with the newer workspace-owned runtime commands, which weakens the truthfulness of the MVP operator story.
- Existing runtime tests validate helper behavior, but Phase 6 still needs a cross-surface contract proving that operators can discover and trust the deploy-run-recover path end to end.

</code_context>

<specifics>
## Specific Ideas

- A strong first slice is one operator-ops summary surface that composes runtime health, beacon, reload, service-install, and lock state into actionable restart or recovery guidance.
- The docs should describe one canonical production loop: install or verify the managed runtime service, inspect runtime health and beacon, review reload or upgrade guidance, take a backup, then restart or recover through the bounded commands.
- Verification should combine runtime summary data and recovery helpers in one test so regressions are caught at the operator contract layer.

</specifics>

<deferred>
## Deferred Ideas

- Full cluster lifecycle orchestration, HA failover, and multi-node operations remain beyond the MVP production boundary.
- Enterprise fleet management, remote rollout orchestration, and centralized audit export are deferred to later enterprise work.
- Deep infrastructure-as-code parity across every deployment target is out of scope for this phase.

</deferred>

---
*Phase: 06-deployment-runtime-and-operator-ops*
*Context gathered: 2026-03-26*
