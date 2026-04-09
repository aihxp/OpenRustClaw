---
phase: 184
requested_reviewers: [claude]
reviewers: [claude]
reviewed_at: 2026-04-09T21:02:11.292Z
plans_reviewed: [184-01-PLAN.md, 184-02-PLAN.md]
---
# Cross-AI Plan Review — Phase 184

## Claude Review

# Cross-AI Review: Phase 184 — Skill Proposal Verification and Reuse

## 184-01-PLAN: Shared Contracts, Durable Storage, and App-Layer Service

### Summary

Solid foundational plan that mirrors the proven learning-candidate pattern from Phase 183. The split between typed contracts, SQLite storage, and an app-layer service with trait-based source abstraction is consistent with established codebase conventions. Scope is well-bounded.

### Strengths

- Reuses the `LearningReviewSource` / `LearningReviewService` pattern directly, reducing design risk
- Proposal artifacts stored outside `skills/` prevents accidental activation (D-05 satisfied)
- TDD approach with lifecycle gate tests catches the most dangerous failure mode early
- Provenance back to learning candidates is a first-class concern, not an afterthought

### Concerns

- **MEDIUM**: No mention of how proposal artifact files are actually written to disk. The app service needs filesystem access, but the trait abstraction doesn't clarify who owns directory creation, path validation, or cleanup on rollback. Risk of path traversal if proposal IDs aren't sanitized.
- **MEDIUM**: Task 1 and Task 2 both touch `crates/app/src/skill_proposals.rs` — Task 1 creates the service and Task 2 creates the store it depends on. The dependency ordering within the plan is inverted; the store should exist before the service tests can run against real persistence.
- **LOW**: Verification report shape isn't specified. If 184-02 needs to display verification results to operators, the contract needs to be concrete enough here to avoid a round-trip redesign.
- **LOW**: No migration rollback story. If the `skill_proposals` table schema needs revision in a future phase, there's no mention of migration versioning strategy (though the existing `crates/db/src/migrate.rs` pattern likely handles this).

### Suggestions

- Swap task ordering: define storage/migration first (current Task 2), then build the app service on top (current Task 1). This matches the actual dependency flow.
- Add path sanitization for proposal artifact directories — reuse the existing path traversal prevention from the security crate.
- Define a minimal `VerificationReport` struct in the types task so 184-02 doesn't have to guess the shape.

### Risk Assessment

**LOW**. The plan follows a proven pattern, scope is tight, and the main risks are ordering/sequencing issues that are easily fixed during execution.

---

## 184-02-PLAN: CLI, Control, MCP Surfaces, and Install Bridge

### Summary

Reasonable operator-surface plan that extends existing CLI/control/MCP patterns. The install bridge through `SkillRegistryMutationService` is the right call. However, the plan is vague on the most critical piece — the actual materialization step that converts a proposal artifact into a workspace skill candidate — and underspecifies the MCP tool contract.

### Strengths

- Install flows through existing `SkillRegistryMutationService` rather than inventing a parallel path
- Thin CLI/control handlers over the app service matches project conventions
- Rollback is explicitly scoped with installed-skill linkage
- Threat model correctly identifies the install-bridge gate as the key safety boundary

### Concerns

- **HIGH**: The materialization step (copying proposal files into `skills/<name>/` so `workspace_skill_candidate` discovers them) is the most error-prone part and is described only as "materialize the proposal into the active skill path." This needs explicit specification: what files are copied, what happens if the target already exists, and how the copy is atomic or rolled back on failure.
- **MEDIUM**: `crates/cli/src/commands/start.rs` is flagged in CLAUDE.md as a "compatibility-heavy surface." Adding HTTP routes and MCP handlers there increases the blast radius. The plan should clarify whether new routes go into a dedicated proposal handler module or inline into the existing monolith.
- **MEDIUM**: MCP tool contract is unspecified. Phase 183 presumably established a pattern; this plan should name the exact tools (e.g., `skill_proposal_list`, `skill_proposal_verify`, `skill_proposal_install`) and their parameter shapes.
- **LOW**: No idempotency story for install. If an operator runs install twice on the same approved proposal, the plan doesn't say whether it's a noop, an error, or a re-install.

### Suggestions

- Specify the materialization step explicitly: which files are copied, directory structure expectations, conflict resolution (reject if skill exists vs. overwrite with provenance update).
- Name the MCP tools and their parameter shapes in the plan so execution doesn't drift.
- Add idempotency behavior: install on an already-installed proposal should return a noop report (matching the `SkillRegistryMutationService::install` "already installed" pattern).
- Consider a thin `proposal_control.rs` module in `crates/cli/src/commands/` rather than inlining everything into `control.rs` and `start.rs`.

### Risk Assessment

**MEDIUM**. The install bridge is the critical path and is underspecified. The plan's success depends on execution-time decisions about file materialization that should be pinned down before implementation starts.

---

## Overall Phase Assessment

The two-plan split is sensible and the wave ordering is correct. The main gap is the materialization bridge between plans — 184-01 stores proposals as inactive files, 184-02 activates them, but neither plan fully specifies the file-to-workspace-skill conversion contract. Pin that down and the phase should land cleanly.

---

## Consensus Summary

### Agreed Strengths
- Single-reviewer artifact: see the completed reviewer section above for the usable strengths signal.

### Agreed Concerns
- No cross-review consensus is available because only one reviewer completed successfully.

### Divergent Views
- No multi-reviewer comparison is available, and no explicit overall risk label was parsed from the completed review.
