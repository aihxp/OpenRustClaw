# Phase 183 Validation

## Goal-backward check

Phase 183 is complete only if all of the following are true:

1. Successful runs, reflections, and relevant audit evidence can become durable learning candidates with provenance, confidence, and explicit review state.
2. Learning candidates can be approved, rejected, superseded, or rolled back without mutating raw lesson files by hand.
3. Promoted lessons still flow through the existing bounded lesson lane and do not silently widen authority.
4. High-impact guidance cannot promote without replay, evaluation, or equivalent review evidence.

## Required evidence

### Storage and lifecycle
- tests prove candidates round-trip with source provenance, review status, and linked evidence
- tests prove candidate status transitions preserve timestamps and reviewer metadata
- tests prove rollback and supersede history remain durable after lesson mutation

### Promotion gating
- tests prove direct reflection promotion no longer writes a lesson immediately
- tests prove higher-impact candidates are blocked when required evidence is missing
- tests prove approved candidates can promote into the existing lesson registry with a stable candidate-to-lesson link

### Operator surfaces
- tests prove CLI or MCP listing returns candidate state, provenance, and evidence
- tests prove operators can approve, reject, supersede, and roll back candidates through shipped control surfaces
- tests prove promoted lesson rollback also updates candidate review state

## Verification commands

- `cargo test -p openrustclaw-core --lib`
- `cargo test -p openrustclaw-db learning_store -- --nocapture`
- `cargo test -p openrustclaw-app learning_review -- --nocapture`
- `cargo test -p openrustclaw-cli orchestrate -- --nocapture`
- `cargo test -p openrustclaw-cli control -- --nocapture`
- `cargo test -p openrustclaw-cli mcp_server_control_tools -- --nocapture`
- `cargo check -p openrustclaw-cli --tests`

## Phase guardrails

- Do not generate or install skill proposals in this phase
- Do not widen default autonomy or God Mode permissions in this phase
- Do not replace the active lesson registry with a second runtime path
- Do not allow silent promotion from reflection output straight into live runtime lessons
