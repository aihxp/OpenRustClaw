---
phase: 87
requested_reviewers: [gemini, claude]
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T19:21:35.239Z
plans_reviewed: [87-01-PLAN.md]
---
# Cross-AI Plan Review — Phase 87

## Gemini Review

# Review of Plan 87-01: Extract the Voice-Call and Channel-Extension Control Route Family

## 1. Summary
Plan 87-01 provides a straightforward and focused approach to resolving the architectural debt in `start.rs`. It accurately targets the phase's core objective: migrating business orchestration for voice-call lifecycles and channel extensions down into the application layer (`openrustclaw-app`). By explicitly defining the creation of a new service boundary, the refactoring of existing handlers into thin adapters, and the inclusion of deterministic testing, the plan establishes a solid foundation for achieving the phase's success criteria.

## 2. Strengths
- **Direct Alignment with Goals:** The steps directly map to the stated success criteria, ensuring the HTTP layer becomes a pure adapter.
- **Test-Driven Verification:** Explicitly requires a deterministic runtime-router test, which is crucial for validating complex lifecycle events (binding, reconnecting, ending) without flaky network dependencies.
- **Appropriate Scope:** The plan correctly limits its focus to the specific route families mentioned in the context, avoiding unnecessary scope creep into already-extracted plugins or non-voice skills.

## 3. Concerns
- **[MEDIUM] Error and Status Mapping:** The context notes that these handlers "already shared the same error/status mapping shape." The plan does not detail how errors will be bubbled up from the new `openrustclaw-app` service and translated into HTTP responses in `start.rs`. There is a risk of unintentionally altering the API contract or losing error context during this transition.
- **[MEDIUM] Service Granularity:** Grouping both "voice-call lifecycle" and "channel-extension control" into a *single* shared application service might violate the Single Responsibility Principle. While they may share underlying helpers currently, their domain logic is distinct. Tightly coupling them in one service could complicate future maintenance.
- **[LOW] API Contract Assurance:** While the plan mentions verifying the route family, it doesn't explicitly mandate that the external HTTP API surface (request/response payloads, headers, status codes) must remain strictly identical before and after the refactor.

## 4. Suggestions
- **Define Error Translation:** Add a specific sub-step to define the error types returned by the new `openrustclaw-app` service and establish a clear mapping to HTTP status codes in `start.rs` to guarantee API consistency.
- **Evaluate Service Boundaries:** Consider creating two distinct services within `openrustclaw-app`—one for `VoiceCallService` and one for `ChannelExtensionService`—rather than forcing them into a single boundary, relying on shared domain models or lower-level helpers where they overlap.
- **Add Contract Verification:** Explicitly state that the verification step must prove the external HTTP API contract (payloads and status codes) is entirely unchanged by this refactoring. 

## 5. Risk Assessment
**Overall Risk Level: LOW**

**Justification:** This is a standard, well-understood architectural refactoring pattern (extracting business logic from controllers to services). The planned changes are structural rather than behavioral. Because the plan includes deterministic testing and targets a specific, bounded area of the codebase, the risk of systemic failure is low. The primary risks involve minor regressions in error handling or API contracts, which are easily mitigated by implementing the suggested contract verification and error-mapping strategies.

---

## Claude Review

The review is complete above. Overall assessment: **LOW risk**, well-scoped extraction that follows established patterns. The minor suggestions are about making the plan more self-documenting rather than fixing design issues.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed on the phase direction and sequencing, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for plan-specific concerns.

### Divergent Views
- Not all reviewers exposed a phase-level risk label; parsed labels: gemini=LOW.
