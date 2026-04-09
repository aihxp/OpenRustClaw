---
phase: 5
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T16:04:55.926Z
plans_reviewed: [05-01-PLAN.md, 05-02-PLAN.md, 05-03-PLAN.md]
---
# Cross-AI Plan Review — Phase 5

## Gemini Review

Here is a cross-AI review of the implementation plans for Phase 5: Email and Voice Communications.

### 05-01-PLAN.md (Email Workflows)

**1. Summary**
This plan focuses on making the existing Gmail Pub/Sub ingress and action paths observable by persisting structured receipts to the workspace and exposing them via the Control UI. It successfully limits scope to observability and trust rather than reinventing the email transport layer, cleanly addressing the COMM-01 requirement.

**2. Strengths**
* **Tight Scope:** Reuses the existing Gmail Pub/Sub parsing and webhook paths rather than attempting a generic mailbox abstraction (aligns with D-02).
* **Pattern Reuse:** Mentions persisting durable operator evidence, correctly matching the execution ledger patterns established in Phase 4.
* **Full Stack:** Touches the backend (`gmail_pubsub.rs`), control API (`start.rs`), and frontend (`control_ui.html`), ensuring the feature is actually usable by operators.

**3. Concerns**
* **Security/PII Leakage (HIGH):** Emails inherently contain sensitive Personally Identifiable Information (PII) and potentially large raw attachments. Persisting raw receipts without a sanitation or redaction strategy risks polluting the local workspace with sensitive operator data.
* **Storage Exhaustion (MEDIUM):** Unbounded persistence of email receipts could consume significant disk space over time, especially if attachments or large threads are base64 encoded into the receipts.

**4. Suggestions**
* Explicitly mandate a sanitization step in `gmail_pubsub.rs` to strip or truncate raw email bodies and drop attachment payloads before writing the receipt to the `.claw` workspace.
* Define a retention policy or a bounded limit (e.g., keeping only the last 100 email receipts) so the workspace storage doesn't grow infinitely.

**5. Risk Assessment**
**MEDIUM.** The implementation path is straightforward, but handling external email data safely requires strict PII guardrails to avoid turning the persistence layer into a security liability.

---

### 05-02-PLAN.md (Voice Communications)

**1. Summary**
This plan hardens the visibility of voice sessions by standardizing how session lifecycle states (healthy, stale, ended, failed) are recorded and surfaced to the operator. By depending on 05-01, it sequences correctly to reuse any UI or API patterns established for communication diagnostics.

**2. Strengths**
* **Safe Sequencing:** Explicitly depends on `05-01`. Since both plans modify `start.rs` and `control_ui.html`, this dependency correctly prevents race conditions or git merge conflicts during autonomous execution.
* **Lifecycle Focus:** Correctly targets the "stale" and "reap" flows, which are historically the hardest parts of telephony to diagnose.
* **Clear State Boundaries:** Focuses on distinct end-reasons (why a session stalled or failed), which directly solves Success Criterion #3.

**3. Concerns**
* **Performance/Payload Size (MEDIUM):** Voice transcripts and artifact metadata can become large. Loading all historical voice sessions into memory to serve the `/control/voice/...` endpoint could stall the control plane if the UI doesn't paginate.
* **Error Handling (LOW):** The plan mentions identifying stale sessions, but it's ambiguous if the UI will allow the operator to actively terminate/reap them or just view them.

**4. Suggestions**
* Specify that the `start.rs` control endpoint must bound the payload size (e.g., returning a summarized list of recent sessions and requiring a separate call to fetch full transcripts).
* Clarify whether the Control UI should include an actionable "Reap Stale Session" button or if it is strictly a read-only diagnostic view for MVP.

**5. Risk Assessment**
**LOW.** The underlying voice capabilities are already shipped in `voice_runtime.rs`. This is primarily a data-shaping and presentation task, which carries minimal architectural risk.

---

### 05-03-PLAN.md (Docs & Verification)

**1. Summary**
This plan closes the phase by ensuring the operator documentation (`README.md`, `quickstart.md`) matches the newly shipped observability features and adds cross-surface E2E verification. It guarantees the milestone ships with verifiable trust loops rather than just backend code.

**2. Strengths**
* **Operator-Centric:** Grounds the phase completion in the `getting-started` flow, ensuring the new diagnostics actually help a first-time operator.
* **Truthful Verification:** Validates the end-to-end operator trust path rather than relying purely on isolated unit tests.
* **Alignment:** Respects the project constraint of "Truth over gloss" by specifically documenting what is shipped.

**3. Concerns**
* **Test Flakiness (MEDIUM):** "Cross-surface communications verification" might inadvertently introduce network-dependent tests if it attempts to hit live Gmail APIs or external voice providers in CI.
* **Scope Creep (LOW):** End-to-end tests for both Voice and Email in a single plan could become a massive testing surface if not tightly constrained.

**4. Suggestions**
* Explicitly state that the integration tests in `tests/integration/src` should utilize local fixtures, mocked webhook payloads, or mock provider responses rather than relying on live network dependencies.
* Ensure the docs provide an explicit troubleshooting command or UI navigation path for when the readiness checks fail (e.g., missing credentials).

**5. Risk Assessment**
**LOW.** Documentation and test finalization are standard wrap-up tasks. As long as external network dependencies are mocked in the test suite, this plan is safe and highly valuable.

---

### Overall Phase 5 Review

The plans collectively meet the phase goals perfectly without over-engineering the transport layers. They abide by the project's directives to avoid scope creep (D-01, D-03) and effectively push the observability of these systems into the Control UI. If the PII/sanitization gaps in Plan 01 and the CI test mocking in Plan 03 are addressed, this phase is well-positioned for a successful, high-quality execution.

---

## Claude Review

# Cross-AI Review: Phase 5 — Email and Voice Communications

## 05-01: Add Durable Operator Evidence for Email Workflows

### Summary
Solid plan that extends the Gmail Pub/Sub channel with persistent receipts and control-plane visibility. The scope is well-bounded to hardening what exists rather than building new email infrastructure. The wave-1 sequencing is correct since voice and docs plans depend on this pattern being established first.

### Strengths
- Reuses the existing `gmail_pubsub.rs` parsing and action infrastructure rather than inventing a new email path
- Follows the Phase 4 operator-evidence pattern (durable receipts → control-plane inspection → UI visibility), maintaining consistency across milestones
- Correctly scoped to Gmail Pub/Sub only, avoiding scope creep into generic mailbox abstractions
- Test task explicitly ties regression coverage to the inspection surface, not just internal persistence

### Concerns
- **MEDIUM**: No mention of storage bounds or retention policy for email receipts. In a busy Gmail integration, unbounded receipt accumulation could grow the `.claw/` workspace indefinitely. The plan should specify a retention window or cap.
- **MEDIUM**: The plan doesn't address what happens to in-flight receipts during runtime restart. If the receipt write is non-atomic with the action, operators could see actions without receipts or receipts without completed actions.
- **LOW**: "Gmail readiness and failure behavior are clear enough for production debugging" is a success criterion but no task explicitly addresses readiness probe improvements in `services.rs`, which the context doc identifies as an existing gap.
- **LOW**: No mention of sensitive data handling — email subjects, sender addresses, and body snippets in receipts could expose PII through the control plane without explicit redaction or access gating.

### Suggestions
- Add an explicit retention/rotation policy for email receipts (e.g., last N entries or time-windowed)
- Include `services.rs` readiness probe hardening as a sub-task or acknowledge it's deferred
- Specify whether receipt persistence is synchronous with action completion or best-effort

### Risk Assessment
**LOW** — The plan is well-scoped and builds on proven patterns. The retention and atomicity gaps are real but unlikely to block MVP credibility.

---

## 05-02: Harden the Voice Communication Trust Path

### Summary
Reasonable plan that tightens the already-rich voice runtime into a clearer operator trust story. The focus on lifecycle state normalization and failure diagnosis is appropriate given the context doc's observation that the voice surface is feature-rich but hard to reason about. The wave-2 dependency on 05-01 is justified since both plans feed the same control UI and should share evidence patterns.

### Strengths
- Correctly prioritizes legibility over new features — the voice runtime already has extensive capability
- Explicit focus on distinguishing healthy/stale/ended/failed states, which is the core operator confusion the context doc identifies
- Shares the control-plane and UI inspection pattern with 05-01, keeping the operator experience consistent across communication lanes
- "Workspace-bounded and durable" constraint prevents scope creep into external monitoring dependencies

### Concerns
- **MEDIUM**: The plan is notably vague compared to 05-01. "Strengthen the persisted voice-session record or derived summaries" doesn't specify what's actually changing — new fields, new files, normalization of existing state, or summary derivation. This leaves significant implementation ambiguity.
- **MEDIUM**: No mention of the compiled-skill voice-call controls from `skills.rs` that the context doc explicitly calls out as part of the voice lane. The plan only touches `voice_runtime.rs` and control surfaces, potentially leaving skill-initiated voice calls without the same trust path.
- **LOW**: The dependency on 05-01 may be overstated. Voice work doesn't technically need email receipts to be done first — they share a UI surface but could be developed in parallel with a merge at the UI layer.
- **LOW**: No mention of reconnect/reap behavior hardening, which the context doc lists as existing voice_runtime capability. If those paths don't produce clear receipts, the trust story has a gap.

### Suggestions
- Add specificity to the first task: enumerate which voice session fields or states need normalization
- Explicitly include or defer `skills.rs` voice-call plugin lifecycle in the trust path
- Consider whether the 05-01 dependency could be relaxed to allow parallel execution, with only the UI task gated on 05-01's control-plane patterns

### Risk Assessment
**MEDIUM** — The plan's vagueness creates risk of under-delivery or scope drift during implementation. The voice runtime is complex and the plan doesn't anchor tightly enough to specific behavioral changes.

---

## 05-03: Align Communications Docs and Verification

### Summary
Clean capstone plan that closes the phase with truthful documentation and cross-surface verification. The wave-3 sequencing is correct — docs and end-to-end tests should reflect the final state of 05-01 and 05-02. The scope is appropriately narrow: update two doc surfaces and add integration tests.

### Strengths
- Explicitly requires docs to describe "actual shipped" behavior, not speculative future state — good discipline
- Cross-surface verification (not just unit tests) matches the phase goal of operator trust
- Touches only `README.md` and `quickstart.md`, avoiding unnecessary doc sprawl
- Key link from README → quickstart ensures the operator story is coherent across entry points

### Concerns
- **MEDIUM**: The plan doesn't mention the Control UI documentation story. Plans 05-01 and 05-02 both add UI visibility, but 05-03 only documents README and quickstart. If the Control UI's communications panel isn't self-documenting or mentioned in docs, operators may not discover it.
- **LOW**: "End-to-end tests that prove the email and voice trust paths remain inspectable from the intended runtime surfaces" is ambitious — true E2E tests against Gmail and voice would require external service mocks or stubs, but the plan doesn't address test infrastructure for this.
- **LOW**: No mention of the `docs/src/` book structure beyond quickstart. If there's a communications or channels section in the mdBook, it should be updated too for consistency.

### Suggestions
- Add a sub-task or note about documenting the Control UI communications panel (even if just a pointer in quickstart)
- Clarify the test strategy: are these mock-based integration tests, or do they require a running runtime with stubbed external services?
- Check whether `docs/src/` has a channels or communications page that also needs updating

### Risk Assessment
**LOW** — Documentation and verification plans carry minimal execution risk. The Control UI gap is a completeness concern, not a blocking issue.

---

## Phase-Level Assessment

### Overall Risk: **LOW-MEDIUM**

The phase is well-structured with correct dependency ordering and consistent scope. The main risks are:

1. **05-02 vagueness** is the biggest concern — the voice plan needs more implementation specificity to avoid either under-delivering or scope-creeping during execution
2. **Retention policy gap** across both 05-01 and 05-02 — neither plan addresses how receipt storage is bounded over time
3. **Compiled-skill voice-call gap** — the context doc explicitly includes skill-initiated voice calls but no plan clearly owns that path

The phase will achieve its stated goals if the implementer fills in the voice plan's ambiguity sensibly and adds storage bounds. The email plan (05-01) is the strongest of the three and sets a good pattern for the rest.

---

## Consensus Summary

### Agreed Strengths
- Reviewers generally agreed the plans are structured and traceable to the phase goal, but they emphasized different strengths.

### Agreed Concerns
- No clearly shared concern theme was detected across reviewers; reviewer-specific concerns are preserved above.

### Divergent Views
- Reviewers disagreed on overall risk level: gemini=MEDIUM, claude=LOW.
