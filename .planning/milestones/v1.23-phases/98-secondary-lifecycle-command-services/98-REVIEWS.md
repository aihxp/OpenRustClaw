---
phase: 98
requested_reviewers: [gemini, claude]
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T19:34:08.746Z
plans_reviewed: [98-01-PLAN.md]
---
# Cross-AI Plan Review — Phase 98

## Gemini Review

# Review of Plan 98-01

## 1. Summary
The plan correctly identifies the high-level goal of extracting business logic from CLI command modules (`channels.rs`, `schedule.rs`, `services.rs`, `control.rs`) into `openrustclaw-app` services, which aligns well with the architectural direction of the Greenfield Conversion. However, the plan is exceedingly brief and lacks the necessary technical depth, architectural specificity, and error-handling strategies required to guide a safe and predictable refactoring effort.

## 2. Strengths
- **Alignment with Architecture:** Accurately targets the specific files mentioned in the phase context and correctly separates concerns (keeping I/O, persistence, and transport in the adapters while moving business rules to the app layer).
- **Scope Clarity:** The boundaries of what is being moved (routing, shaping, health monitoring) versus what is staying (metadata reads, transport) are logically sound.

## 3. Concerns
- **[HIGH] Lack of Interface Definition:** The plan does not specify the names, signatures, data structures, or trait definitions of the new app-side services to be created. This leaves too much ambiguity for the implementation phase.
- **[HIGH] Missing Error Handling Strategy:** There is no mention of how domain errors will be modeled in `openrustclaw-app` and how they will be propagated and mapped to user-facing CLI output or exit codes in the adapters.
- **[MEDIUM] Vague Testing Strategy:** "Verify the extracted lane with app coverage and CLI library verification" is insufficient. It does not dictate how the boundary between the CLI and the app layer should be mocked or tested, nor does it explicitly require unit tests for the new app services.
- **[MEDIUM] Dependency Management:** The plan fails to detail how dependencies (like database connections, configuration state, or API clients) will be passed into or injected into these new app-side services.
- **[LOW] Incremental Delivery:** The plan groups four distinct domains (channels, schedules, services, control) into a single step. Refactoring them all at once increases the risk of merge conflicts and integration regressions.

## 4. Suggestions
- **Define Service Contracts:** Explicitly list the proposed traits, structs, and public method signatures that will be added to `openrustclaw-app` (e.g., `ChannelRouter`, `SchedulePlanner`).
- **Specify Error Types:** Define a custom error enum for the new app services (e.g., `AppServiceError`) and mandate how the CLI adapters will handle these errors.
- **Detail the Testing Plan:** Require specific unit tests for the new business logic in `openrustclaw-app` and integration tests ensuring the CLI adapters correctly wire up the new services. Mention the use of mocks if applicable.
- **Break Down the Implementation:** Split Step 2 into distinct, sequential sub-steps for each domain (1. Extract Channels, 2. Extract Schedules, etc.) to allow for atomic, verifiable commits.
- **Clarify State Injection:** Briefly state whether these services will be stateless functions, structs holding state, or traits implemented by a core context object.

## 5. Risk Assessment
**HIGH** 

While the conceptual goal is perfectly aligned with the project's direction, the plan itself is too underdeveloped to be safely executed. The lack of detail regarding data structures, error handling, and testing makes it highly probable that the resulting implementation will be inconsistent, buggy, or require significant rework to meet the project's quality standards. The plan needs to be expanded into a detailed technical design before execution begins.

---

## Claude Review

The review is provided above. The plan is directionally correct but underspecified — the main gaps are missing concrete extraction targets, no service interface design, and vague verification criteria.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed on the phase direction and sequencing, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for plan-specific concerns.

### Divergent Views
- Not all reviewers exposed a phase-level risk label; parsed labels: gemini=HIGH.
