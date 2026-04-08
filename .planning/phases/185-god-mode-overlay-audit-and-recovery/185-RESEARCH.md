# Phase 185 Research: God Mode Overlay, Audit, and Recovery

## Current state

OpenRustClaw already has:
- an enterprise autonomy override in `crates/cli/src/commands/enterprise_autonomy.rs`
- baseline runtime-policy capture and restore on enable, disable, and kill-switch
- active and recent run summaries for full-autonomy executions
- a shipped enterprise control UI and inspect/admin summaries
- durable learning candidates and skill proposals with rollback-capable lifecycle services

OpenRustClaw does not yet have:
- first-class `God Mode` branding and reporting on top of the override lane
- TTL or expiry handling for the stronger lane
- explicit God Mode provenance on learned artifacts
- quarantine-specific controls for God Mode-derived learning candidates or skill proposals

## Phase 185 decisions

### D-185-01: Treat God Mode as a named overlay on the existing enterprise autonomy seam

Phase 185 should promote the existing full-autonomy override into a first-class God Mode contract instead of creating a second override implementation.

Why:
- the current seam already captures baseline policy, override policy, and kill-switch history
- `GOD-04` explicitly rejects implicit inheritance, which is easier to guarantee with one override path
- a second runtime stack would create policy drift and rollback ambiguity

### D-185-02: Add explicit expiry and baseline restoration to the manifest lifecycle

God Mode should store TTL or expiry metadata in the manifest and enforce expiry through the same Rust-owned control path used for enable, disable, and summary.

Why:
- `GOD-02` requires TTL or session boundaries
- expiry is only meaningful if it restores the saved baseline automatically
- enforcing expiry in the control layer keeps the behavior auditable and independent from any one UI client

### D-185-03: Label runs by God Mode, not just by yolo/autonomy inference

Run summaries and operator-facing reports should carry an explicit God Mode label.

Why:
- operators should not have to infer the strongest lane from raw autonomy fields
- `GOD-03` requires prominent labeling
- explicit labeling allows later correlation with learned artifacts and quarantine actions

### D-185-04: Persist God Mode provenance on learned artifacts

Learning candidates and skill proposals should persist whether they originated under God Mode, plus quarantine metadata.

Why:
- later review needs to distinguish normal learning from stronger-lane learning
- existing artifact types already have durable storage and rollback logic, so adding provenance there fits established patterns
- quarantine must be durable and auditable rather than a transient UI flag

### D-185-05: Quarantine should be operational

Quarantining a God Mode-derived artifact should use the existing rollback/deactivate seams when the artifact is active.

Why:
- a promoted lesson or installed skill remains live unless quarantine also triggers containment
- rollback and deactivation are already shipped and auditable
- Phase 185 should add as little new operational machinery as necessary

## Implementation shape

### God Mode manifest and report

Extend the enterprise-autonomy manifest/report with:
- a `mode_label` such as `God Mode`
- explicit scope metadata
- TTL input and `expires_at`
- expiry detection and an `expired` event
- report strings that surface God Mode directly

### Learned artifact provenance

Extend learning candidates and skill proposals with:
- `god_mode_origin`
- quarantine metadata (`quarantined_at`, `quarantined_by`, `quarantine_reason`)

Use this metadata to:
- label list and inspect payloads
- preserve durable audit history
- drive containment on quarantine

### Operator controls

Add control flows for:
- enabling God Mode with TTL
- observing expiry and baseline restore in summaries
- quarantining learning candidates
- quarantining skill proposals

Expose these through:
- CLI control commands
- HTTP control handlers
- MCP tools where the current learning/proposal lifecycle already exists

## Risks and mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| God Mode stays enabled after an operator forgets to disable it | Default runtime drifts into implicit high-power mode | Persist `expires_at`, enforce expiry in control/report paths, and restore baseline automatically |
| God Mode-derived artifacts become indistinguishable from normal artifacts | Unsafe learning gets harder to review | Persist explicit `god_mode_origin` on learning candidates and proposals |
| Quarantine is only cosmetic | Active lessons or installed skills remain live after quarantine | Reuse lesson deactivation and proposal rollback when quarantining active artifacts |
| Renaming breaks existing enterprise control routes | Regressions in shipped admin flows | Keep route family and governance scope stable while upgrading labels and report content |

## Recommended plan split

- `185-01`: God Mode manifest/report updates, TTL expiry enforcement, baseline restore, and report/UI naming updates
- `185-02`: learned-artifact provenance, quarantine controls, CLI/HTTP/MCP lifecycle exposure, and milestone closeout verification
