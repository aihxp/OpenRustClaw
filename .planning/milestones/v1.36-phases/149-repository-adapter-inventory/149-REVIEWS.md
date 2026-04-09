---
phase: 14
requested_reviewers: [gemini, claude]
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T16:33:49.514Z
plans_reviewed: [149-01-PLAN.md]
---
# Cross-AI Plan Review — Phase 14

## Gemini Review

Here is the review of the provided implementation plan.

### Summary
The plan accurately reflects the goals and user decisions for Phase 149, correctly targeting the transition of persistence-heavy command-local helpers into a formalized repository-adapter inventory. However, the plan is exceedingly abstract. It essentially restates the objective without defining the concrete actions, deliverables, or formats required to achieve that objective. Because the phase is designated as a "planning-contract level" effort, the output must be a tangible artifact (like a markdown document or an architecture decision record), but the plan fails to specify what will actually be created or where it will be stored.

### Strengths
*   **Strong Alignment:** The plan strictly adheres to the user decision to keep this phase at the "planning-contract level" rather than diving into immediate source code changes.
*   **Targeted Scope:** Correctly identifies the need to focus on specific, known hotspots (`start.rs`, `runtime.rs`, `skills.rs`, `inspect.rs`) rather than attempting a generalized overhaul of the entire legacy command layer at once.

### Concerns
*   **HIGH: Missing Deliverables Definition:** The plan uses verbs like "Define", "Tie", and "Verify" without specifying the output medium. It is unclear if the output should be an Architecture Decision Record (ADR), a new planning document (e.g., `REPOSITORY_ADAPTERS.md`), or inline code comments mapping the future traits.
*   **MEDIUM: Vague Methodology:** There is no step detailing *how* the inventory will be conducted. Relying on "real persistence-heavy hotspots" is subjective. The plan needs a step to audit specific types of operations (e.g., direct SQLite queries, direct file system I/O for state/cache).
*   **MEDIUM: Lack of Architectural Guidance:** While identifying the legacy hotspots is important, the plan does not include a step to define the shape of the "successor ownership slice." It should outline the proposed trait boundaries or adapter interfaces that will eventually replace the legacy code.

### Suggestions
*   **Specify the Output Artifact:** Update Step 1 to explicitly state what document is being created or updated (e.g., "Create `docs/adr/00X-repository-adapter-inventory.md` outlining the current persistence hotspots and the target repository trait interfaces").
*   **Define Discovery Criteria:** Add a sub-step detailing how to identify a hotspot (e.g., "Audit the command modules for direct `rusqlite` usage, `std::fs` operations for configuration, and skill cache manipulations").
*   **Draft the Target Interfaces:** Modify Step 2 to include drafting the proposed Rust trait definitions for the new repository layer (e.g., `AuditLogRepository`, `SkillCacheRepository`) to ensure the successor slice is well-defined before implementation begins.
*   **Establish a Verification Artifact:** Update Step 3 to require a specific verification step, such as ensuring all identified hotspots in the specified files map to exactly one proposed repository interface.

### Risk Assessment
**MEDIUM**
The primary risk is ambiguity. Because the plan does not define the concrete deliverables or the methodology for the inventory, the agent executing this plan may struggle to determine when the task is "done" or may produce an output format that is not useful for the subsequent implementation phases. Adding concrete document targets and explicit interface-drafting steps will lower this risk to LOW.

---

## Claude Review

# Review: Plan 149-01 — Repository Adapter Inventory

## Summary

This is a lightweight planning-contract phase that defines an inventory of persistence-heavy hotspots and maps them to successor repository-adapter ownership. The plan is intentionally narrow — three steps, no source changes — which is appropriate for a scoping/inventory phase. However, the plan is so thin that it's difficult to evaluate whether it will actually produce a useful artifact. The steps read more like goals than actionable work items.

## Strengths

- Correctly scoped as a planning-contract phase rather than attempting source changes
- Grounds the inventory in real hotspot files (`start.rs`, `runtime.rs`, `skills.rs`, `inspect.rs`) rather than abstract categories
- Covers the right persistence domains: sqlite, workspace files, audit logs, runtime config, compiled-skill cache, registry ownership
- Avoids scope creep — doesn't try to implement the adapters, just inventory them

## Concerns

- **HIGH**: The three steps are nearly identical restatements of the same goal ("define inventory", "tie to hotspots", "verify path exists"). There's no concrete deliverable format specified — what does the inventory artifact actually look like? A table? A markdown file? Annotations in code?
- **MEDIUM**: No enumeration methodology. How will persistence-heavy helpers be identified — grep for `sqlx`/`rusqlite`/`libsql` calls? Manual reading? The plan should specify the discovery approach so the inventory is reproducible and complete.
- **LOW**: The phase section wasn't found in the milestone roadmap, which raises a traceability question — is this phase properly anchored in the current planning deck?

## Suggestions

- Replace the three overlapping steps with concrete deliverables: (1) scan persistence call sites in the five named command modules, (2) produce a table mapping each call site to a persistence domain and proposed repository trait, (3) identify which existing crate (`db`, `memory`, a new `repositories` crate) should own each adapter
- Specify the output artifact format — likely a markdown inventory table with columns: file, function/block, persistence domain, current implementation, proposed successor owner
- Add a step to cross-reference against `crates/db/` and `crates/memory/` to identify which repository contracts already exist vs. which need to be created

## Risk Assessment

**LOW** — This phase can't break anything since it produces no source changes. The main risk is that the deliverable is too vague to be actionable for subsequent phases. Tightening the step definitions and specifying the artifact format would make this a solid foundation for the actual repository-lift work.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed on the phase direction and sequencing, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for plan-specific concerns.

### Divergent Views
- Reviewers disagreed on overall risk level: gemini=MEDIUM, claude=LOW.
