# Phase 185 Validation

## Goal-backward check

Phase 185 is complete only if all of the following are true:

1. Operators can explicitly enable a distinct God Mode lane instead of inheriting it from the normal runtime path.
2. God Mode activation persists scope and expiry metadata, supports kill-switch recovery, and restores the baseline runtime policy on disable or expiry.
3. God Mode runs and learned artifacts are visibly labeled and preserve durable provenance.
4. God Mode-derived learning candidates and skill proposals can be quarantined through shipped control surfaces without deleting history.
5. The default runtime path remains trust-first after God Mode disable or expiry and does not retain approval bypasses or stronger runtime grants.

## Required evidence

### God Mode control lifecycle
- tests prove enable captures baseline policy and TTL metadata
- tests prove summary or lifecycle refresh expires God Mode and restores the baseline policy
- tests prove kill-switch still restores baseline and records affected runs
- tests prove operator-facing reports identify the lane as God Mode

### Learned artifact provenance and quarantine
- tests prove learning candidates persist God Mode provenance and quarantine metadata
- tests prove skill proposals persist God Mode provenance and quarantine metadata
- tests prove quarantining a promoted lesson deactivates the active lesson
- tests prove quarantining an installed proposal triggers rollback or equivalent containment

### Operator surfaces
- tests prove CLI/control output exposes God Mode labels and quarantine state
- tests prove HTTP control and MCP handlers can quarantine learned artifacts
- tests prove inspect or enterprise admin summaries describe the stronger lane as God Mode

## Verification commands

- `cargo test -p openrustclaw-core --lib`
- `cargo test -p openrustclaw-db learning_store -- --nocapture`
- `cargo test -p openrustclaw-db skill_proposal_store -- --nocapture`
- `cargo test -p openrustclaw-app learning_review -- --nocapture`
- `cargo test -p openrustclaw-app skill_proposals -- --nocapture`
- `cargo test -p openrustclaw-cli control -- --nocapture`
- `cargo test -p openrustclaw-cli inspect -- --nocapture`
- `cargo test -p openrustclaw-cli mcp_server_control_tools -- --nocapture`
- `cargo check -p openrustclaw-cli --tests`

## Phase guardrails

- Do not add a second God Mode runtime stack
- Do not widen the default trust-first runtime path outside explicit God Mode activation
- Do not silently retain God Mode authority after disable, kill-switch, or expiry
- Do not delete learned artifacts during quarantine; preserve history and provenance
