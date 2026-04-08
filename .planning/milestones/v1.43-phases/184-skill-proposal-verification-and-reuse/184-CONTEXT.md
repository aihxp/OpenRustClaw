# Phase 184: Skill Proposal Verification and Reuse - Context

**Gathered:** 2026-04-08
**Status:** Ready for planning

<domain>
## Phase Boundary

Turn repeated successful work into reviewable reusable-skill proposals without letting candidate text or generated skill content become active by default. This phase is about durable proposal storage, human-readable proposal artifacts, verification, approval, and routing approved proposals through the existing skill install and compile spine. It is not yet about God Mode authority changes or widening the default runtime lane.

</domain>

<decisions>
## Implementation Decisions

### Proposal lifecycle
- **D-01:** Phase 184 should add a first-class durable skill-proposal queue instead of writing generated skills straight into `skills/` or the compiled cache.
- **D-02:** Skill proposals should preserve explicit provenance back to the learning candidate or lesson that justified them.
- **D-03:** Proposal lifecycle should stay explicit and inspectable with states such as `pending_review`, `approved`, `rejected`, `verified`, `installed`, `superseded`, and `rolled_back` or equivalent bounded transitions.

### Human-readable, diffable artifacts
- **D-04:** Proposal content should be materialized as human-readable files under a dedicated non-active workspace lane so operators can diff and inspect it before install.
- **D-05:** Proposed skill content must remain inactive until it passes verification and explicit approval; proposal files are not the same thing as installed skills.
- **D-06:** Verification should reuse the existing compile and skill-policy spine wherever possible instead of inventing a second parser or scanner.

### Install and reuse path
- **D-07:** Approved proposals should enter the live system by flowing through the existing workspace-skill install and compile path, not by bypassing `skills` mutation services.
- **D-08:** Verification and install actions should preserve durable provenance so operators can trace an installed skill back to its source proposal, candidate, and lesson.

### Operator control and safety
- **D-09:** Operators need list, inspect, diff, review, verify, approve, install, supersede, and rollback controls through shipped CLI/control/MCP lanes.
- **D-10:** Proposal approval should not silently bypass sensitive capability policy, signature requirements, or compile-time blocking behavior already enforced by the skill subsystem.

### Scope guardrails
- **D-11:** Phase 184 should not automatically infer or auto-install arbitrary skills from free-form transcripts; proposals remain bounded, inspectable artifacts.
- **D-12:** Phase 184 should not widen runtime authority or approvals; God Mode remains Phase 185.

### the agent's Discretion
- Exact proposal artifact layout under the workspace control directory
- Whether verification state is best persisted as compile-summary metadata, dedicated verification records, or both
- The cleanest operator-facing boundary between "approve", "verify", and "install" actions

</decisions>

<specifics>
## Specific Ideas

- Reuse the Phase 183 learning-candidate review seam as the stable origin for proposal provenance.
- Materialize proposals into a non-active directory such as `.claw/control/skill-proposals/<proposal-id>/` so the content is readable and diffable without becoming an installed workspace skill.
- Keep proposal install as an explicit copy or promotion step into `skills/<name>/` followed by the existing skill mutation and compile path.
- Use the existing compile overview and blocked-artifact behavior as the verification substrate so proposal review sees the same safety posture the runtime will enforce after install.

</specifics>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Milestone and phase contract
- `.planning/PROJECT.md` - Current milestone intent and active requirements for v1.43
- `.planning/REQUIREMENTS.md` - `SKIL-01` through `SKIL-03` define the committed skill-proposal scope
- `.planning/ROADMAP.md` - Phase 184 goal, dependency boundary, and success criteria
- `.planning/STATE.md` - Current milestone position after Phase 183 completion

### Prior phase outputs
- `.planning/phases/183-learning-candidate-review-and-lesson-promotion/183-01-SUMMARY.md` - durable candidate queue and review service
- `.planning/phases/183-learning-candidate-review-and-lesson-promotion/183-02-SUMMARY.md` - control, MCP, and orchestration wiring for reviewable learning output

### Existing codebase seams
- `crates/app/src/learning_review.rs` - learning-candidate lifecycle and promotion-policy seam
- `crates/app/src/skill_registry_mutation.rs` - existing workspace and registry skill install/update/uninstall orchestration
- `crates/app/src/skill_control.rs` - typed control surface for skill inspection, compile, install, and verify actions
- `crates/app/src/compiled_skill_overview.rs` - compiled artifact inspection and reference-reading seam
- `crates/cli/src/commands/skills.rs` - skill install, compile, verify, and compiled-artifact helpers
- `crates/cli/src/commands/control.rs` - workspace-scoped control services and durable review patterns
- `crates/cli/src/commands/start.rs` - control routes and MCP registration points
- `crates/db/src/learning_store.rs` - durable review queue, evidence, and promotion-history pattern to mirror

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/app/src/learning_review.rs`: already centralizes review, approval, promotion, and rollback logic for learned artifacts and can supply proposal provenance
- `crates/app/src/skill_registry_mutation.rs`: already owns the trustworthy install/update/uninstall spine that Phase 184 should reuse for activation
- `crates/cli/src/commands/skills.rs`: already knows how to compile, verify, inspect, and materialize compiled skill artifacts
- `crates/app/src/compiled_skill_overview.rs` and `crates/app/src/compiled_skill_mcp.rs`: already expose bounded compiled-artifact summaries appropriate for proposal verification and operator review

### Established Patterns
- Rust owns durable state, review transitions, and policy gates
- Control and MCP layers stay thin over typed app services
- Reviewable artifacts keep provenance, evidence, and explicit lifecycle state instead of mutating live runtime assets directly

### Integration Points
- Proposal creation should attach to approved learning candidates or promoted lessons rather than inventing a parallel source of truth
- Proposal verification should reuse skill compilation and policy checks before install
- Proposal install should land in the existing workspace-skill mutation lane so runtime behavior matches normal skills

</code_context>

<deferred>
## Deferred Ideas

- Automatic mining of arbitrary transcripts into skills without review
- Marketplace publication or remote registry push for approved proposals
- God Mode overlays, authority widening, or approval bypasses - Phase 185
- Rich graph UI for proposal lineage - later milestone work

</deferred>

---

*Phase: 184-skill-proposal-verification-and-reuse*
*Context gathered: 2026-04-08*
