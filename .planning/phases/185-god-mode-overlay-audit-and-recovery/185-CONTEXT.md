# Phase 185: God Mode Overlay, Audit, and Recovery - Context

**Gathered:** 2026-04-08
**Status:** Ready for planning

<domain>
## Phase Boundary

Add a distinct `God Mode` operator lane that grants full autonomy, full access, and full power without weakening the default trust-first runtime path.

This phase is about explicit activation, TTL or expiry handling, baseline restore, kill-switch recovery, God Mode audit labeling, and quarantine-ready handling for learned artifacts produced under the stronger lane. It is not about a second runtime stack, new learning queues, or broader UI redesign.

</domain>

<decisions>
## Implementation Decisions

### Reuse the existing autonomy override seam
- **D-01:** Phase 185 should extend the shipped `enterprise_autonomy` stack instead of introducing a second parallel God Mode runtime.
- **D-02:** God Mode remains an explicit enterprise/operator override layered on top of the existing runtime autonomy policy and restored back to the saved baseline on disable, kill-switch, or expiry.

### Explicit scope and expiry
- **D-03:** God Mode activation must persist explicit scope metadata and an expiry boundary so the stronger lane cannot remain implicit forever.
- **D-04:** TTL expiry should be enforceable from the same Rust-owned control path that currently enables, disables, and kill-switches full autonomy.

### Audit labeling and provenance
- **D-05:** God Mode runs should be labeled as such in reports and control surfaces instead of only inferring the lane from `autonomy_level = yolo` and `approval_policy = none`.
- **D-06:** Learned artifacts produced under God Mode should preserve durable God Mode provenance so operators can inspect and reason about their origin later.

### Quarantine and recovery
- **D-07:** Learning candidates and skill proposals need explicit quarantine metadata and operator actions so God Mode-derived artifacts can be contained without deleting history.
- **D-08:** Quarantining a promoted lesson or installed skill should also trigger the existing rollback or deactivate seam so quarantine is operational, not cosmetic.

### Scope guardrails
- **D-09:** Phase 185 must not widen the default trust-first runtime path for non-God-Mode operation.
- **D-10:** Phase 185 must not add a second long-lived governance model; richer multi-operator approval chains stay future milestone work.

### the agent's Discretion
- The exact metadata shape used to label God Mode-derived artifacts
- Whether TTL expiry is stored as an absolute timestamp, duration metadata, or both
- Which shipped operator surfaces need the strongest God Mode naming updates in this phase

</decisions>

<specifics>
## Specific Ideas

- Rebrand the current `enterprise full autonomy` lane as `God Mode` at the report and operator-surface level while keeping the governance scope and existing route family stable.
- Persist God Mode activation metadata under `.claw/control/enterprise/full-autonomy.json` with TTL, scope, and baseline policy details.
- Teach summary and mutation paths to auto-expire God Mode, restore the baseline runtime policy, and append an `expired` audit event.
- Mark God Mode-derived learning candidates and skill proposals with explicit provenance plus quarantine metadata so review and rollback can remain durable and auditable.

</specifics>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Milestone and phase contract
- `.planning/PROJECT.md`
- `.planning/REQUIREMENTS.md` - `GOD-01` through `GOD-04`
- `.planning/ROADMAP.md`
- `.planning/STATE.md`

### Prior phase outputs
- `.planning/phases/183-learning-candidate-review-and-lesson-promotion/183-01-SUMMARY.md`
- `.planning/phases/183-learning-candidate-review-and-lesson-promotion/183-02-SUMMARY.md`
- `.planning/phases/184-skill-proposal-verification-and-reuse/184-01-SUMMARY.md`
- `.planning/phases/184-skill-proposal-verification-and-reuse/184-02-SUMMARY.md`

### Existing code seams
- `crates/cli/src/commands/enterprise_autonomy.rs` - existing override, baseline restore, kill-switch, and run-summary seam
- `crates/cli/src/commands/control.rs` - runtime autonomy policy, learning candidate, and skill proposal control services
- `crates/cli/src/commands/start.rs` - control routes and MCP registration points
- `crates/cli/src/commands/control_ui.html` - shipped operator-facing enterprise autonomy surface
- `crates/cli/src/commands/inspect.rs` - enterprise admin and autonomy inspection summaries
- `crates/app/src/learning_review.rs` - learned-artifact promotion and rollback seam
- `crates/app/src/skill_proposals.rs` - proposal verification, install, and rollback seam
- `crates/db/src/learning_store.rs` - durable learning candidate storage and history
- `crates/db/src/skill_proposal_store.rs` - durable proposal storage and history

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `enterprise_autonomy` already captures baseline policy, kill-switch events, and full-autonomy run summaries.
- Learning candidates already preserve autonomy level and can roll back promoted lessons.
- Skill proposals already preserve provenance, verification state, install lineage, and rollback.

### Established Patterns
- Stronger runtime lanes are opt-in and operator-gated.
- Reviewable artifacts keep durable history instead of being deleted in place.
- CLI, HTTP control, inspect, and MCP layers stay thin over typed Rust services.

### Integration Points
- God Mode should sit on top of the existing runtime autonomy policy and control registry.
- God Mode-derived learning candidates and skill proposals should be labeled at creation time and remain quarantine-capable through the same control surfaces that already manage them.
- Enterprise admin, inspect, and control UI surfaces should describe the stronger lane explicitly as God Mode so operators can distinguish it from normal runtime autonomy.

</code_context>

<deferred>
## Deferred Ideas

- Multi-operator or dual-approval activation for God Mode - future governance work
- Unattended campaign orchestration beyond simple TTL windows - future milestone work
- Broad dashboard redesign for learning artifact lineage - later UX milestone
- A second separate God Mode runtime stack - explicitly out of scope

</deferred>

---

*Phase: 185-god-mode-overlay-audit-and-recovery*
*Context gathered: 2026-04-08*
