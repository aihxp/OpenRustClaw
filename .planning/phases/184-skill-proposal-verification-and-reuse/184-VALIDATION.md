# Phase 184 Validation

## Goal-backward check

Phase 184 is complete only if all of the following are true:

1. Reviewed learning output can become durable reusable-skill proposals with explicit provenance and inactive artifact files.
2. Skill proposals stay human-readable and diffable without becoming active workspace skills by default.
3. Operators can approve, reject, supersede, verify, install, and roll back proposals through shipped control lanes.
4. Approved proposals only become active by flowing through the existing workspace skill install and compile path.

## Required evidence

### Storage and artifact lifecycle
- tests prove proposals round-trip with provenance, status, artifact path, and verification metadata
- tests prove proposal files are created outside the active `skills/` root and remain inspectable
- tests prove supersede and rollback transitions preserve durable proposal history

### Verification and install gating
- tests prove unapproved proposals cannot install
- tests prove unverified proposals cannot install
- tests prove proposal verification uses the existing compile and policy path and persists its outcome
- tests prove installed skills preserve provenance back to the proposal and source learning artifact

### Operator surfaces
- tests prove CLI or MCP listing returns proposal status, provenance, and verification summary
- tests prove operators can review and verify proposals without manually editing proposal files
- tests prove install copies or materializes proposal content into the active skill path and reuses existing install behavior

## Verification commands

- `cargo test -p openrustclaw-core --lib`
- `cargo test -p openrustclaw-db skill_proposal_store -- --nocapture`
- `cargo test -p openrustclaw-app skill_proposals -- --nocapture`
- `cargo test -p openrustclaw-cli control -- --nocapture`
- `cargo test -p openrustclaw-cli skills -- --nocapture`
- `cargo test -p openrustclaw-cli mcp_server_control_tools -- --nocapture`
- `cargo check -p openrustclaw-cli --tests`

## Phase guardrails

- Do not auto-install generated skills when a proposal is queued
- Do not bypass the existing skill mutation, compile, or capability-policy path during install
- Do not widen autonomy or God Mode permissions in this phase
- Do not treat raw transcripts or unchecked text blobs as installed skills
