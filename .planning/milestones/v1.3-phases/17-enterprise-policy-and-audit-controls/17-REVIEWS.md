---
phase: 17
requested_reviewers: [gemini, claude]
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T17:07:59.059Z
plans_reviewed: [17-01-PLAN.md, 17-02-PLAN.md, 17-03-PLAN.md]
---
# Cross-AI Plan Review — Phase 17

## Gemini Review

Here is the cross-AI plan review for Phase 17: Enterprise Policy and Audit Controls.

### 1. Summary
The plans for Phase 17 provide a pragmatic and well-scoped approach to unifying enterprise policy and audit controls. By aggregating existing durable state (YAML, TOML) rather than reinventing the configuration system, the plans successfully align with the project's goal of maintaining an honest, file-backed source of truth. The sequencing is logical, moving from the internal policy manifest (17-01) to the export bundle (17-02) and concluding with operator documentation (17-03). The explicit deferral of compliance-targeted exports and UI work keeps the implementation focused and achievable.

### 2. Strengths
*   **Architectural Honesty:** The plans adhere strictly to the principle of using existing files as the source of truth rather than creating an opaque new database or conflicting state.
*   **Clear Sequencing:** The dependencies are explicitly defined (17-01 → 17-02 → 17-03), ensuring the foundation is built before the export logic is introduced.
*   **Scope Discipline:** The plans specifically avoid scope creep by acknowledging deferred ideas (like tenant-aware separation and PDF/CSV compliance exports).
*   **Actionable Export Metadata:** D-06 is well-represented in Plan 17-02; returning export metadata (paths, counts) instead of raw files significantly improves the operator experience.

### 3. Concerns
*   **HIGH:** **Atomicity of Multi-File Updates (Plan 17-01):** The `/control/enterprise/policy` PUT route must update multiple independent files (e.g., `.claw/control/enterprise/` manifest, runtime YAML, runtime config TOML). If a write succeeds for the YAML but fails for the TOML, the system is left in a partially applied, inconsistent state. The plan does not specify a rollback or transactional strategy for these writes.
*   **MEDIUM:** **Export Bundle Permissions and Lifecycle (Plan 17-02):** The plan introduces a file-backed JSON export bundle but does not define where these are stored, their default file permissions, or how they are cleaned up. Since these contain sensitive enterprise audit evidence, improper permissions or unbounded accumulation could lead to data leaks or disk exhaustion.
*   **MEDIUM:** **Concurrency During Export (Plan 17-02):** Gathering data from multiple live files (access registry, browser backend audit, tool history) to create a single snapshot could result in a torn or inconsistent state if those files are actively being appended to by other threads during the read operation.
*   **LOW:** **Input Validation Constraints (Plan 17-01):** The plan does not explicitly call out the need for strict schema validation on the PUT route. Given that this route touches critical autonomy and browser backend configurations, malformed input could corrupt the underlying TOML/YAML files.

### 4. Suggestions
*   **Implement a "Write-Ahead" or Staged Apply (17-01):** When updating the unified policy, validate all inputs in memory first, then write to temporary files, and finally swap them atomically (or as close to atomically as possible) to prevent partial policy application.
*   **Define Export Security (17-02):** Explicitly enforce strict file permissions (e.g., `0600`) on the generated audit export bundles. Designate a specific, protected directory for these exports (e.g., `.claw/control/enterprise/exports/`) and consider adding a maximum retention count to prevent disk bloat.
*   **Add Snapshot Consistency Locks (17-02):** Ensure that the read operations generating the audit export use appropriate file locks or read-only snapshot mechanisms so the resulting bundle represents a coherent point-in-time state.
*   **Expand Documentation (17-03):** Update the documentation tasks to include troubleshooting steps for policy update failures, explicit details on where export bundles are saved, and how operators should secure or rotate those files.

### 5. Risk Assessment
**MEDIUM** 
**Justification:** While the architectural direction is highly aligned with the project's existing patterns, the introduction of a single route that orchestrates writes across multiple critical configuration files introduces a tangible risk of state corruption via partial failures. Additionally, handling sensitive audit data requires explicit security guardrails that are currently missing from the plan descriptions. Addressing the atomicity and file-permission concerns during implementation will easily mitigate these risks down to LOW.

---

## Claude Review

# Phase 17: Enterprise Policy and Audit Controls — Cross-AI Review

## Plan 17-01: Unified Enterprise Policy Surface

### Summary
Solid plan that consolidates scattered enterprise policy state into one typed surface with read/write capability. The approach of aggregating existing durable config files rather than introducing a new persistence layer is the right call.

### Strengths
- Reuses existing durable sources (access registry, autonomy YAML, browser TOML) instead of adding a new store
- File-backed manifest for mobile/audit settings keeps the pattern consistent
- Typed policy report gives operators one place to look

### Concerns
- **MEDIUM** — No mention of concurrent write handling. If two operators PUT policy updates simultaneously, last-write-wins on file-backed config could silently drop changes.
- **MEDIUM** — No validation story for policy updates. What happens if an operator sets an invalid approval policy value or contradictory settings? The plan says "update" but not "validate before applying."
- **LOW** — The plan modifies 5 files but the task breakdown only has 2 tasks. The mapping of which changes land in which task is loose — could lead to a sprawling first task.

### Suggestions
- Add explicit input validation for policy PUT payloads before writing to durable config
- Consider an optimistic-lock header (e.g., `If-Match` with a policy version/hash) to catch concurrent updates
- Split Task 1 into "manifest + aggregation helpers" and "mod.rs wiring" for cleaner commits

### Risk Assessment
**LOW** — The scope is well-bounded and builds on proven patterns. The concerns are about robustness, not feasibility.

---

## Plan 17-02: Enterprise Audit Export Bundle

### Summary
Straightforward extension that bundles policy state and recent evidence into a durable JSON file. The design is honest about what it exports and where it writes — good operator UX.

### Strengths
- Export includes both policy snapshot and evidence, making it self-contained
- Returns metadata (path, counts, timestamp) rather than raw content — operators know what they got
- Reuses existing durable sources rather than duplicating data

### Concerns
- **HIGH** — No mention of sensitive data handling in exports. Enterprise audit bundles likely contain operator identities, access scopes, and action logs. If the export lands as plaintext JSON in a world-readable directory, that's a security gap. The plan should specify file permissions and warn about sensitive content.
- **MEDIUM** — No size bounding on "recent evidence." If an operator has months of audit history, the export could be unexpectedly large. The plan should define a default window (e.g., last 30 days) with an optional override.
- **LOW** — Export path selection is unspecified. Does the operator choose it? Is there a default under `.claw/`? The plan says "writes to disk" but not where.

### Suggestions
- Write exports with restrictive file permissions (0600) and document the sensitivity
- Default to a bounded time window for evidence inclusion; accept an optional `since` parameter
- Specify the default export directory (e.g., `.claw/exports/enterprise/`) and allow override via query param

### Risk Assessment
**MEDIUM** — The sensitive-data-in-export concern needs addressing before shipping. The functional design is sound but the security posture of the output artifact is unspecified.

---

## Plan 17-03: Documentation and Verification

### Summary
Clean closure plan. Docs-only with verification evidence. Appropriately scoped for a wave-3 task.

### Strengths
- Docs target the right files (README for discovery, production.md for workflow)
- Explicitly links docs to shipped routes rather than abstract descriptions
- Verification artifact closes the phase truthfully

### Concerns
- **MEDIUM** — No mention of documenting failure modes. Enterprise operators need to know what happens when policy writes fail, when exports fail mid-write, or when evidence sources are empty. Happy-path docs alone aren't sufficient for enterprise trust.
- **LOW** — No example payloads or CLI snippets mentioned. Enterprise policy docs without concrete examples are harder to trust and use.

### Suggestions
- Include at least one example request/response for both the policy route and the export route
- Document error responses and common failure scenarios (missing config, empty evidence, disk full)
- Add a brief "what's NOT in scope" note so enterprise operators don't expect compliance-grade exports yet

### Risk Assessment
**LOW** — Documentation plans rarely carry execution risk. The concern about failure-mode coverage is about quality, not feasibility.

---

## Overall Phase Assessment

| Dimension | Rating |
|---|---|
| **Scope** | Well-bounded, no creep |
| **Dependency ordering** | Correct (01 → 02 → 03) |
| **Goal achievement** | Plans cover all 3 success criteria |
| **Biggest gap** | Export security posture (Plan 02) |
| **Overall risk** | **LOW-MEDIUM** |

The phase is well-structured and achievable. The single item I'd flag as must-fix before execution is the sensitive data handling in Plan 02 — enterprise audit exports containing identity and action data should not land with default file permissions and no sensitivity warning. Everything else is refinement.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed on the phase direction and sequencing, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for plan-specific concerns.

### Divergent Views
- Not all reviewers exposed a phase-level risk label; parsed labels: gemini=MEDIUM.
