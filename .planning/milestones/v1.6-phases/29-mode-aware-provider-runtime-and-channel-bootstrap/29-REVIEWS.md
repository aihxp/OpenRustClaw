---
phase: 29
requested_reviewers: [gemini, claude]
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T17:35:10.813Z
plans_reviewed: [29-01-PLAN.md, 29-02-PLAN.md, 29-03-PLAN.md]
---
# Cross-AI Plan Review — Phase 29

## Gemini Review

# Cross-AI Plan Review: Phase 29

Here is the structured feedback for the Phase 29 implementation plans.

## 1. Summary
The Phase 29 plans provide a logical, well-sequenced approach to closing the "honest setup" gap. By first extending the durable state model (29-01) and then wiring in existing provider/runtime health checks (29-02) and channel probes (29-03), the plans align perfectly with the phase goals. They successfully avoid inventing new validation heuristics by reusing the shipped observability and health surfaces. However, the plans currently under-specify the user experience around network latency, error handling during these new synchronous validation steps, and data serialization compatibility.

## 2. Strengths
- **Reuses Existing Truth:** Plans 29-02 and 29-03 correctly reuse `runtime_health_status` and `channel_probes_status` instead of creating redundant onboarding-specific validation logic.
- **Solid Sequencing:** Plan 29-01 prepares the data model first, ensuring the subsequent validation steps have a concrete place to persist their outcomes.
- **Honest Contract:** The plans explicitly mandate recording `warning` or `blocked` states rather than assuming a successful configuration write equates to a working system, fulfilling the core value of the milestone.

## 3. Concerns
- **HIGH: Synchronous Network Latency & CLI UX:** Validating API keys and channel probes inherently requires network calls. The plans do not specify how this latency will be communicated to the operator (e.g., spinners, progress bars). Without UI feedback, the CLI may appear to hang during setup.
- **HIGH: Hard-Blocking on Secondary Failures:** Plan 29-03 does not specify if a failed channel probe (e.g., Slack API is temporarily down) will hard-block the entire onboarding process. Channels are often secondary to core local runtime functionality and should likely result in a `Warning`/`Degraded` state rather than a complete block.
- **MEDIUM: Serialization Compatibility:** Plan 29-01 does not explicitly mention how the new `SetupState` fields will handle existing serialized state files on disk from earlier versions (e.g., missing `#[serde(default)]`), which could cause panics or parse errors for operators upgrading to this version.
- **MEDIUM: Missing Timeout Boundaries:** Relying on external APIs for validation during setup requires strict timeout boundaries to prevent indefinite hanging if a provider endpoint is unresponsive.

## 4. Suggestions
- **Define the Outcome Schema:** In 29-01, explicitly define the Rust enum for bootstrap outcomes (e.g., `Ready`, `Degraded(String)`, `Blocked(String)`) to ensure rich error messages are captured alongside the status.
- **Ensure Backward Compatibility:** In 29-01, ensure the new fields on `SetupState` use `#[serde(default)]` so that existing state files can be safely deserialized without the new bootstrap outcome arrays/maps.
- **Add UX Indicators & Timeouts:** In 29-02 and 29-03, explicitly require the use of CLI spinners or status indicators during network calls, and enforce strict timeouts (e.g., 5-10 seconds) on validation probes.
- **Graceful Degradation:** In 29-03, clarify that channel validation failures should result in a `Warning` or `Degraded` state that allows the operator to finish onboarding and repair the channel later, rather than hard-blocking the setup completion.
- **Offline Local Model Support:** Ensure that in 29-02, validation for local models (like Ollama) gracefully handles scenarios where the operator has no outbound internet access.

## 5. Risk Assessment
**Risk Level: MEDIUM**

**Justification:** The architectural approach is highly sound because it relies on existing, proven validation boundaries. The risk elevates to MEDIUM solely due to the human-factors impact of introducing synchronous network validation into a previously offline/fast onboarding flow. If timeouts, backward compatibility in state serialization, and CLI UX (spinners) are not explicitly handled during implementation, it could lead to frustrating user experiences (hanging CLIs or locked onboarding states). Addressing the suggestions above will reduce this to a LOW risk execution.

---

## Claude Review

The review is complete above. The key takeaway is a **MEDIUM** overall risk driven by one shared gap: all three plans assume runtime health and channel probe surfaces are reachable during onboarding, but onboarding typically runs *before* the runtime is started. Adding a `DeferredToFirstStart` outcome variant would close this cleanly.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed on the phase direction and sequencing, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for plan-specific concerns.

### Divergent Views
- Risk posture could not be parsed reliably from the reviewer output; inspect the sections above.
