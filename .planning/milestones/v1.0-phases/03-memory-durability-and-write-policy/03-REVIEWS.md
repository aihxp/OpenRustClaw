---
phase: 3
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T15:59:04.941Z
plans_reviewed: [03-01-PLAN.md, 03-02-PLAN.md, 03-03-PLAN.md]
---
# Cross-AI Plan Review — Phase 3

## Gemini Review

# Cross-AI Plan Review: Phase 3 (Memory Durability and Write Policy)

### 1. Summary
The proposed plans for Phase 3 effectively translate the goal of durable, policy-governed memory into actionable, well-sequenced steps. The roadmap logically progresses from establishing the core policy primitives and tool boundary enforcement (Plan 01), to exposing that new metadata in operator surfaces (Plan 02), and finally aligning documentation and end-to-end tests (Plan 03). The approach correctly targets the `MemoryStoreTool` schema for enforcement rather than relying purely on prompt engineering, which aligns well with the project's trust-first product constraints.

### 2. Strengths
- **Logical Sequencing**: The wave-based execution (enforce backend -> expose in UI/CLI -> document and verify E2E) minimizes integration friction and prevents UI changes from blocking on unfinished backend logic.
- **Appropriate Enforcement Boundary**: Enforcing the policy at the `memory_store` tool schema level (Plan 01) rather than just tweaking the system prompt ensures robust, programmatic enforcement that the LLM cannot easily bypass.
- **Graceful Rejection Handling**: Returning a non-error refusal for blocked writes (Plan 01) is an excellent design choice. It prevents the assistant's execution loop from crashing while still strictly enforcing the memory policy.
- **Consistent Surface Parity**: Plan 02 ensures that both the CLI and the web-based Control UI are updated in tandem, preventing the kind of operator surface drift the project has worked hard to eliminate in earlier milestones.

### 3. Concerns
- **Database Schema Migration [HIGH]**: None of the plans explicitly address modifying the SQLite database schema or creating a migration script to durably store the new `policy_basis`, `reason`, or `source_classification` metadata. If the existing `memory_store` persistence layer lacks a JSON blob column or specific columns for this data, Plan 01 will fail during the storage phase.
- **Backward Compatibility of Tool Schema [MEDIUM]**: Modifying the `memory_store` tool schema to require a policy basis and reason could break existing custom workflows, tests, or legacy integrations that rely on the old arguments. It is not specified if these new fields are strictly required or if there is a fallback mechanism.
- **Visibility of Blocked Writes [LOW]**: Plan 01 specifies returning a non-error refusal to the model, but it does not state if or how these blocked write attempts are logged. Operators might be confused if the assistant "ignores" an implicit memory and the refusal is completely invisible outside of debug logs.

### 4. Suggestions
- **Add a Persistence/Migration Task (Plan 01)**: Explicitly include a task to audit the storage layer (e.g., SQLite schema) and add a database migration script to support the new metadata fields, or explicitly verify that an existing extensible metadata column will be used.
- **Define the Schema Transition (Plan 01)**: Specify whether the new tool arguments (`policy_basis`, `reason`) are mandatory. Ensure internal test fixtures and default prompts are updated atomically alongside the schema change to prevent CI breaks.
- **Log Policy Denials (Plan 01/02)**: Add a lightweight audit log or debug trace when a memory write is blocked by policy. This will make the "inspectable" requirement even stronger by letting operators see *attempted* writes that the policy caught.
- **Shift E2E Verification (Plan 03 to 02)**: Consider moving the "cross-surface verification" task from Plan 03 into Plan 02. The end-to-end integration tests should ideally be written and passed alongside the CLI/UI exposure to prove the surfaces work, leaving Plan 03 strictly for documentation alignment.

### 5. Risk Assessment
**MEDIUM**

**Justification**: The architectural approach is conceptually sound and aligns perfectly with the phase context and prior platform constraints. However, the omission of database schema evolution (migrations) is a critical implementation gap that will likely cause the first plan to fail when the code attempts to persist the new metadata to SQLite. Explicitly addressing the storage layer will easily lower this risk to LOW.

---

## Claude Review

# Cross-AI Review: Phase 3 — Memory Durability and Write Policy

## 03-01: Enforce explicit assistant memory-write policy

### Summary
Solid plan that attacks the right problem — uncontrolled memory writes — at the right layer (tool boundary enforcement backed by shared policy primitives). The three-task structure (policy primitives → tool gate → tests) is clean and correctly ordered.

### Strengths
- Enforces at the tool boundary (`MemoryStoreTool`) rather than relying on prompt-only guidance — correct architectural choice
- Requires the model to declare a policy basis, making writes auditable by design
- Preserves existing SQLite backend instead of over-engineering a new storage layer
- Write-basis metadata stored alongside content enables downstream inspection (Plan 02)

### Concerns
- **MEDIUM**: No mention of backward compatibility for existing stored memories that lack write-basis metadata. Plan 02 will surface this metadata in inspection tools — what happens when operators inspect pre-policy memories?
- **MEDIUM**: The boundary between "durable user fact" and "ephemeral detail" is inherently fuzzy. The plan defers exact classification to the implementer ("at the agent's discretion") which is appropriate, but there's no fallback for when the model misclassifies a write basis. A blocked write is unrecoverable if the model picks the wrong policy category.
- **LOW**: No discussion of whether `core_memory_update` (mentioned in CONTEXT as a second write surface) is also gated, or only `memory_store`. If both are write paths, both need the policy gate.
- **LOW**: Prompt guidance update is bundled into the tool-enforcement task rather than called out separately. Prompt drift is a distinct failure mode from code regression.

### Suggestions
- Add an explicit migration note: pre-policy memories get a sentinel write-basis value (e.g., `"legacy"`) so inspection surfaces don't crash or show blanks.
- Clarify whether `core_memory_update` is in scope or explicitly deferred.
- Consider a soft-deny mode for the first release: log blocked writes with reasons rather than silently dropping, so operators can tune the boundary.

### Risk Assessment
**LOW** — Well-scoped, correctly layered, and the main risk (classification fuzziness) is inherent to the domain rather than a plan defect.

---

## 03-02: Make memory-write decisions inspectable to operators

### Summary
Straightforward operator-visibility plan that correctly depends on 03-01's metadata being stored. The scope is right — surface existing metadata rather than build a governance product. The two-task split (CLI + control/UI) is clean.

### Strengths
- Builds directly on 03-01's stored metadata rather than inventing a parallel inspection mechanism
- Covers both CLI and browser control surfaces for consistent operator experience
- Scoped to inspection, not governance — avoids scope creep into retention policies or compliance tooling

### Concerns
- **MEDIUM**: `control_ui.html` is listed as a modified file, implying direct HTML changes for memory inspection UX. If this file is already large or complex, the plan should specify whether this is a small metadata display addition or a significant UI change. Risk of under-estimating frontend effort.
- **MEDIUM**: No mention of filtering or searching by write-basis. If an operator has hundreds of memories, seeing metadata per-entry is necessary but not sufficient — they'll want to filter to "show me all explicit-request memories" or "show me blocked attempts."
- **LOW**: The plan says "regression coverage for operator-visible memory write inspection" in artifacts but the tasks don't include a dedicated test task. Tests appear assumed rather than planned.

### Suggestions
- Add a third task for inspection-specific test coverage rather than relying on it being implicit.
- Consider adding a `--write-basis` filter flag to the CLI memory commands (minimal effort, high debugging value).
- Clarify the `control_ui.html` change scope — is it a column addition to an existing table, or a new panel?

### Risk Assessment
**LOW** — The plan is well-bounded and the dependency on 03-01 is correct. Main risk is under-specifying the control UI work.

---

## 03-03: Align docs and verification with the stricter memory contract

### Summary
Clean closeout plan that ties documentation to enforcement. Correctly positioned as wave 3 depending on both prior plans. The cross-surface verification task adds real value beyond what 03-01's unit-level tests cover.

### Strengths
- Explicitly removes misleading quickstart examples — addresses a real trust gap where docs imply permissive storage
- Cross-surface tests (write + search + inspection) verify the full contract rather than isolated layers
- Two-task structure is minimal and appropriate for a docs-and-verification closeout

### Concerns
- **MEDIUM**: "Cross-surface verification" is vague about what "cross-surface" means concretely. Does it mean: (a) write via tool → inspect via CLI → confirm metadata matches, or (b) write via tool → search via `memory_search` → confirm search results respect policy? Both matter, but the plan should specify which flows are covered.
- **LOW**: No mention of updating the `docs/src/` book index or navigation if the memory guide structure changes significantly.
- **LOW**: The verification task and the docs task are independent but ordered sequentially (tests first, docs second). They could be parallelized if needed.

### Suggestions
- Enumerate 2-3 specific cross-surface test scenarios (e.g., "explicit remember → store → CLI inspect shows basis", "ephemeral write attempt → blocked → no entry in timeline").
- Check whether any other doc pages reference memory behavior (e.g., architecture docs, API reference) and flag them for alignment.

### Risk Assessment
**LOW** — Appropriate scope for a phase closeout. The main risk is under-specifying the cross-surface test matrix, which could lead to shallow coverage that misses the point.

---

## Overall Phase Assessment

### Phase-Level Risk: **LOW**

The three plans form a coherent progression: enforce → inspect → document. The dependency chain (01 → 02 → 03) is correct and the scope stays bounded to the stated goal. No plan attempts to solve enterprise retention, compliance, or advanced memory extraction — those are correctly deferred.

### Phase-Level Concerns

1. **MEDIUM**: The `core_memory_update` write path is mentioned in the context but never explicitly addressed across any of the three plans. If it bypasses the new policy gate, the entire phase has a hole.

2. **MEDIUM**: Backward compatibility for pre-policy memories is not addressed in any plan. This will surface as a concrete problem in Plan 02 when inspection tools encounter memories without write-basis metadata.

3. **LOW**: The phase treats the LLM's policy-basis declaration as trustworthy. A model could game the gate by always claiming "explicit_request." The phase context acknowledges this ("prefer under-storing over over-storing") but no plan includes monitoring or alerting for suspicious classification patterns. This is acceptable for MVP but should be noted as known debt.

### Does the phase achieve its goals?

Yes. The three success criteria from the phase definition are directly addressed:

| Phase Success Criterion | Covered By |
|---|---|
| Memory survives restart under policy boundary | 03-01 (enforcement) + existing SQLite durability |
| Writes only under explicit policy | 03-01 (tool gate) |
| Operators can inspect what/why/recalled | 03-02 (inspection) + 03-03 (docs) |

---

## Consensus Summary

### Agreed Strengths
- **Appropriate Enforcement Boundary**: Enforcing the policy at the `memory_store` tool schema level (Plan 01) rather than just tweaking the system prompt ensures robust, programmatic enforcement that the LLM cannot easily bypass.
- **Graceful Rejection Handling**: Returning a non-error refusal for blocked writes (Plan 01) is an excellent design choice. It prevents the assistant's execution loop from crashing while still strictly enforcing the memory policy.
- **Consistent Surface Parity**: Plan 02 ensures that both the CLI and the web-based Control UI are updated in tandem, preventing the kind of operator surface drift the project has worked hard to eliminate in earlier milestones.

### Agreed Concerns
- **Database Schema Migration [HIGH]**: None of the plans explicitly address modifying the SQLite database schema or creating a migration script to durably store the new `policy_basis`, `reason`, or `source_classification` metadata. If the existing `memory_store` persistence layer lacks a JSON blob column or specific columns for this data, Plan 01 will fail during the storage phase.
- **Backward Compatibility of Tool Schema [MEDIUM]**: Modifying the `memory_store` tool schema to require a policy basis and reason could break existing custom workflows, tests, or legacy integrations that rely on the old arguments. It is not specified if these new fields are strictly required or if there is a fallback mechanism.
- **Visibility of Blocked Writes [LOW]**: Plan 01 specifies returning a non-error refusal to the model, but it does not state if or how these blocked write attempts are logged. Operators might be confused if the assistant "ignores" an implicit memory and the refusal is completely invisible outside of debug logs.

### Divergent Views
- Reviewers disagreed on overall risk level: gemini=MEDIUM, claude=LOW.
