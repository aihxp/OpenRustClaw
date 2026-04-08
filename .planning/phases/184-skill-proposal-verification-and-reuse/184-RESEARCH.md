# Phase 184 Research: Skill Proposal Verification and Reuse

## Current state

OpenRustClaw already has:
- durable learning-candidate review and promotion in `crates/app/src/learning_review.rs`
- typed install and update orchestration in `crates/app/src/skill_registry_mutation.rs`
- typed skill control surfaces in `crates/app/src/skill_control.rs`
- compile, inspect, and verify helpers in `crates/cli/src/commands/skills.rs`
- compiled-artifact overview and MCP exposure in `crates/app/src/compiled_skill_overview.rs` and `crates/app/src/compiled_skill_mcp.rs`

OpenRustClaw does not yet have:
- a durable skill-proposal queue with explicit review and verification state
- human-readable proposal artifacts that stay separate from active workspace skills
- provenance from installed skills back to the learning candidate or lesson that proposed them
- a control flow that verifies and approves proposals before they enter the existing install path

## Phase 184 decisions

### D-184-01: Add a first-class SQLite skill-proposal store

Phase 184 should add Rust-owned durable proposal storage instead of encoding proposal state only in files under `skills/` or in ad hoc JSON payloads.

Why:
- proposal review, verification, and install state need durable lifecycle metadata that does not belong inside `SKILL.md` alone
- Phase 183 already established the pattern of keeping learned artifacts in SQLite-backed review queues
- provenance, verification reports, and install lineage must survive file copies and workspace restarts

### D-184-02: Keep proposal content file-backed but inactive

Proposal content should be written into a dedicated proposal directory under workspace control state, not directly into `skills/`.

Why:
- `SKIL-02` requires proposals to remain human-readable and diffable before activation
- proposal files should be inspectable without the runtime treating them as installed skills
- a non-active directory lets the system compile and review proposals without implicitly installing them

### D-184-03: Reuse approved learning artifacts as the proposal source of truth

Phase 184 should treat approved learning candidates and promoted lessons as the durable provenance anchors for proposal generation.

Why:
- Phase 183 already added the review and evidence bridge between successful work and reusable guidance
- proposal generation should build on reviewed artifacts rather than raw transcripts
- `SKIL-03` explicitly requires durable provenance back to the source candidate or lesson

### D-184-04: Treat verification as compile-and-policy validation

Proposal verification should reuse the existing skill compiler, compiled-artifact status, and capability-policy checks rather than inventing a second "proposal validator."

Why:
- the runtime already trusts the skill compile and verification spine
- blocked-artifact behavior and capability checks are the real safety boundary before install
- one validation path avoids drift between proposal review and production skill behavior

### D-184-05: Activation must flow through the existing install spine

Approved and verified proposals should become active by first materializing a workspace skill and then routing through the current skill mutation/install path.

Why:
- `SKIL-03` requires approved proposals to use the existing compile/install path
- `SkillRegistryMutationService` already owns workspace skill discovery, DB persistence, and post-install compile behavior
- this keeps approved proposals operationally identical to other installed skills

### D-184-06: Separate review, verify, and install actions

Proposal lifecycle should distinguish operator approval from technical verification and from final install.

Why:
- "approved" answers whether an operator wants the skill
- "verified" answers whether the proposal compiles and passes policy checks
- "installed" answers whether the approved, verified proposal was actually activated

## Implementation shape

### Durable proposal contract

Add shared types for:
- proposal status and source kind
- proposal content and artifact paths
- verification reports and compile summaries
- provenance refs to source learning candidates and lessons
- review, verify, install, supersede, and rollback requests

### Storage

Add SQLite tables for:
- `skill_proposals`
- `skill_proposal_history`

These should track:
- stable proposal id
- namespace or workspace scope
- skill name and target path
- source learning candidate id and optional lesson id
- lifecycle status
- proposal summary and rationale
- proposal artifact path
- verification status and report
- installed skill name or installed skill record id
- created, updated, reviewed, verified, installed, superseded, and rolled-back timestamps

### Proposal files

Materialize files under a dedicated control directory such as:
- `.claw/control/skill-proposals/<proposal-id>/SKILL.md`
- optional companion metadata or diff summary files if needed

This keeps proposals:
- readable
- diffable
- outside the active `skills/` discovery root until install

### Review and verification service

Add an app-layer service that can:
- queue a proposal from reviewed learning artifacts
- list and inspect proposals with provenance and file locations
- review or supersede a proposal
- verify a proposal by compiling the proposed skill artifact and persisting the result
- install a proposal by materializing it into `skills/` and invoking the existing skill mutation/install path
- roll back a proposal by uninstalling the resulting skill or marking the proposal inactive

### Operator surfaces

Extend existing CLI and MCP control surfaces so operators can:
- queue a skill proposal
- inspect the proposal file and provenance
- approve, reject, or supersede it
- run verification and see compile/policy results
- install or roll back the approved proposal through shipped control lanes

## Risks and mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Proposal files become active just by being generated | Violates `SKIL-02` | Store proposal artifacts outside `skills/` and require explicit install |
| Verification path diverges from production install behavior | Approved proposals fail after activation | Reuse the existing compiler, capability policy, and workspace install path |
| Installed skills lose provenance back to the reviewed learning artifact | Weakens trust and future rollback | Persist source candidate, lesson, and proposal ids in proposal history and install metadata |
| Proposal approval bypasses sensitive-capability policy | Unsafe skills activate under normal mode | Re-run existing skill policy and compile gates during verification and install |

## Recommended plan split

- `184-01`: shared proposal contracts, durable SQLite proposal queue, file-backed inactive proposal artifacts, and app-layer review/verification service
- `184-02`: CLI/control/MCP proposal flows, install bridge into the existing skill mutation path, rollback handling, and provenance-rich operator inspection
