---
phase: 21
requested_reviewers: [gemini, claude]
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T16:45:17.233Z
plans_reviewed: [21-01-PLAN.md, 21-02-PLAN.md, 21-03-PLAN.md]
---
# Cross-AI Plan Review — Phase 21

## Gemini Review

Here is the structured cross-AI review of the implementation plans for Phase 21.

### 1. Summary

The plans for Phase 21 successfully map out a cohesive, bounded approach to improving enterprise audit retention and review. They correctly sequence the work from foundational data model changes (21-01) to data aggregation and summary generation (21-02), and finally to UI surfacing and documentation (21-03). The plans adhere well to the architectural constraints outlined in the context, specifically avoiding external GRC integration, preventing the creation of new manifests, and relying on Rust-owned typed contracts rather than ad-hoc UI file scraping.

### 2. Strengths

- **Consolidated State:** By updating `crates/cli/src/commands/enterprise_policy.rs` directly, the plans avoid configuration fragmentation and keep enterprise governance under a single truth boundary.
- **Architectural Discipline:** Plan 21-02 explicitly requires using typed Rust summaries rather than having the HTML/JS layer scrape raw bundle files, ensuring a strong, testable boundary between the backend and the UI.
- **Appropriate Scoping:** The plans strictly adhere to the "bounded retention and recent-history packaging" constraint, resisting the urge to over-engineer a full compliance archive.
- **Full Lifecycle Coverage:** Plan 21-03 explicitly includes updates to `README.md` and production documentation, ensuring the new capabilities are discoverable and maintainable.

### 3. Concerns

- **(HIGH) Performance and Memory Boundaries in 21-02:** The plan requires generating a typed review summary of "recent high-signal enterprise events." Without explicit size limits, pagination, or streaming during aggregation, a highly active enterprise instance could cause memory spikes or timeout the `/control/ui` request when generating this summary.
- **(MEDIUM) Retention Cleanup Execution in 21-01:** The plan mentions "export cleanup behavior" but does not define *when* this cleanup occurs. If cleanup happens synchronously during a new export or a UI request, it could cause unexpected latency.
- **(MEDIUM) Backwards Compatibility in 21-02:** The plan does not explicitly state how the system should handle older, existing audit bundles that lack the new governance and supervision context when generating review summaries.
- **(LOW) Input Validation in 21-01:** The plan lacks explicit mention of validation for the new retention settings (e.g., preventing negative retention days or excessively large review limits).
- **(LOW) Partial Failures in 21-02:** If the system attempts to bundle governance, policy, and supervision history, and one of those data stores is temporarily locked or corrupted, it's unclear if the entire audit export fails or degrades gracefully.

### 4. Suggestions

- **Define Cleanup Triggers:** In Plan 21-01, explicitly define whether retention cleanup runs as a background worker task, on startup, or synchronously during the creation of a new export bundle. (A background task or startup check is recommended).
- **Enforce Aggregation Limits:** In Plan 21-02, explicitly mandate bounded queries (e.g., `LIMIT 1000` or a strict time window) when pulling "recent operator history" into the typed summary to guarantee `O(1)` memory usage regardless of instance age.
- **Handle Legacy Bundles:** In Plan 21-02, add a requirement to ensure the typed review summary gracefully handles `Option<None>` or default values for older audit bundles missing the new data fields.
- **Specify UI Rendering Constraints:** In Plan 21-03, ensure the Control UI limits the number of recent exports rendered in the DOM to match the "review limits" defined in 21-01 to prevent browser lag.

### 5. Risk Assessment

**Risk Level: MEDIUM**

**Justification:** The architectural direction is solid and the scope is well-controlled. The medium risk rating stems entirely from the operational execution details—specifically, the potential for unbounded memory usage during summary generation (21-02) and undefined latency characteristics during retention cleanup (21-01). If the implementation explicitly bounds the data aggregation and handles legacy data gracefully, the risk drops to LOW. The plans are ready for implementation provided those constraints are kept in mind during execution.

---

## Claude Review

# Cross-AI Review: Phase 21 — Enterprise Audit Retention and Review Packaging

## 21-01: Deepen Enterprise Audit Retention Policy

**Summary:** Straightforward plan to add retention settings to the existing enterprise policy manifest. Correctly scoped to one file and avoids creating parallel config surfaces.

**Strengths:**
- Reuses existing `enterprise_policy.rs` instead of introducing new manifests
- Bounded scope — retention limits and export cleanup, not a compliance product

**Concerns:**
- **MEDIUM** — No mention of migration/defaults for existing policies missing the new retention fields. Operators upgrading need sane defaults without manual intervention.
- **LOW** — "Export cleanup behavior" is vague — does this mean deleting old exports on disk? If so, destructive file operations need care (e.g., path traversal safety, confirmation).

**Suggestions:**
- Specify default retention values explicitly in the plan
- Clarify whether cleanup is automatic or operator-triggered

**Risk Assessment:** **LOW** — Small surface, single file, well-bounded.

---

## 21-02: Package Governance and Autonomy Evidence for Review

**Summary:** Enriches audit bundles with governance/supervision context and adds a typed review summary endpoint. The plan correctly insists on Rust-owned contracts over file scraping, which aligns with project conventions.

**Strengths:**
- Typed review summary backed by runtime structs — avoids brittle UI-side parsing
- Consolidates governance (Phase 20) and supervision (Phase 18) evidence into one story
- Touches the right files: policy, inspect, and start (for routes)

**Concerns:**
- **MEDIUM** — No specification of what "high-signal enterprise events" means. Without a concrete definition, implementation will either under-deliver or scope-creep.
- **MEDIUM** — The review summary endpoint isn't named. Should be specified (e.g., `/control/enterprise/audit/review`) so 21-03 can consume it without ambiguity.
- **LOW** — Bundle size could grow unboundedly if governance event history isn't capped. The retention limits from 21-01 should explicitly govern this.

**Suggestions:**
- Define the event types that qualify as "high-signal" (e.g., policy mutations, autonomy escalations, God Mode activations)
- Name the review summary route explicitly
- Confirm 21-01's retention limits apply to the enriched bundle contents

**Risk Assessment:** **LOW-MEDIUM** — Sound architecture, but the vague "high-signal" definition could cause scope drift during implementation.

---

## 21-03: Surface Enterprise Audit Review in Control UI

**Summary:** Adds a review panel to the existing Enterprise Admin UI surface and updates docs. Depends on 21-01 and 21-02 delivering their contracts.

**Strengths:**
- Renders typed data from 21-02 rather than scraping files — correct layering
- Includes docs and verification artifacts
- Scoped to one UI panel addition, not a redesign

**Concerns:**
- **MEDIUM** — `control_ui.html` is already a large single-file UI surface. Adding another panel increases the maintenance burden. No mention of whether this is becoming unwieldy.
- **LOW** — "Enterprise admin inputs can manage the stronger audit retention settings" overlaps with whatever mutation surface already exists from 21-01's policy updates. Risk of duplicate controls.
- **LOW** — No mention of access control on the review panel — should it require enterprise admin role?

**Suggestions:**
- Confirm the retention settings mutation reuses the existing enterprise policy mutation flow rather than adding a parallel one
- Verify the review panel respects the same access controls as the existing Enterprise Admin surface

**Risk Assessment:** **LOW** — Straightforward UI addition assuming 21-01 and 21-02 deliver clean contracts.

---

## Overall Phase Assessment

**Risk: LOW.** The phase is well-scoped, correctly ordered (policy → data → UI), and avoids compliance overreach. The main gap across all three plans is **insufficient specificity on data contracts** — retention defaults, event type definitions, and endpoint names should be pinned before implementation to prevent drift. The deferred items list is sensible and shows discipline.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed on the phase direction and sequencing, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for plan-specific concerns.

### Divergent Views
- Risk posture could not be parsed reliably from the reviewer output; inspect the sections above.
