---
phase: 112
requested_reviewers: [gemini, claude]
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T19:59:26.806Z
plans_reviewed: [112-01-PLAN.md]
---
# Cross-AI Plan Review — Phase 112

## Gemini Review

Here is the review for Plan 112-01:

### 1. Summary
Plan 112-01 is currently a high-level strategic outline rather than an executable engineering plan. While it correctly identifies the goal of migrating the Control UI serving and wiring to the native `openrustclaw-gateway` to escape the legacy bootstrap layer, it lacks the concrete technical specifics required to actually implement the change. It does not address how static assets will be served, how API and WebSocket routes will be mapped, or how security concerns like CORS and authentication will be maintained during the transition.

### 2. Strengths
*   **Clear Alignment with Phase Goals:** The plan correctly targets the core objective of decoupling the UI from the legacy bootstrap layer and moving it to the native gateway.
*   **Focus on Compatibility:** Explicitly calls out the requirement to preserve the existing shipped UI contract, ensuring no regressions in operator experience.
*   **Identifies Live Wiring:** Acknowledges that this isn't just about static files, but also about the live wiring (API/WebSockets) that powers the UI.

### 3. Concerns
*   **HIGH - Lack of Executable Steps:** The steps are phrased as ongoing design tasks ("Define native ownership...", "Define how UI actions...") rather than concrete implementation actions. An engineer or autonomous agent cannot execute "Define..." without further specification.
*   **HIGH - Missing Security & Middleware Context:** Moving the UI serving path introduces risks around Cross-Origin Resource Sharing (CORS), Content Security Policy (CSP), and authentication middleware. The plan completely omits how these will be configured in the new native gateway.
*   **HIGH - Unclear Asset Delivery Mechanism:** The plan does not specify *how* the HTML/JS/CSS assets will be served by the Rust gateway (e.g., embedded into the binary via `include_dir!`/`rust-embed`, or served from the local file system via a framework like Axum/Tower).
*   **MEDIUM - Missing Protocol Details:** There is no mention of how real-time communication (presumably WebSockets or Server-Sent Events) will be routed and maintained under the new gateway route contracts.
*   **MEDIUM - No Verification Strategy:** "Verify that the roadmap preserves compatibility" is a goal, not a testing strategy. There are no steps for manual verification, automated UI testing, or verifying that the legacy bootstrap code can be safely deleted.

### 4. Suggestions
*   **Specify the Serving Mechanism:** Explicitly state the technical approach for serving the static UI assets from `openrustclaw-gateway` (e.g., "Implement an Axum `ServeDir` service for `/ui`" or "Use `rust-embed` to serve the UI from the compiled binary").
*   **Map the Routes:** Create a clear mapping of legacy UI routes and API endpoints to their new native gateway counterparts.
*   **Address Middleware:** Add explicit steps to configure CORS, CSP, and authentication/session middleware on the new gateway UI routes to ensure security parity with the legacy system.
*   **Include Real-Time Wiring:** Detail how WebSocket connections or live event streams from the UI will attach to the new gateway handlers.
*   **Define Testing and Cleanup:** Add concrete steps to:
    1. Run E2E tests against the new gateway UI.
    2. Deprecate and remove the legacy UI serving code from the old bootstrap layer.
    3. Verify production asset building (e.g., Webpack/Vite integration with the Rust build).

### 5. Risk Assessment
**HIGH**

**Justification:** The plan in its current state is un-implementable. It serves as a statement of intent but lacks the technical architecture, security considerations, and step-by-step execution details required to safely transition the Control UI to the new gateway without causing widespread breakages or security regressions. The plan must be expanded into concrete engineering tasks before execution begins.

---

## Claude Review

# Review: Phase 112-01 — Control UI Serving and Native Delivery Alignment

## Summary

This is a thin, directional plan that names the right ownership transfer (Control UI serving from legacy bootstrap to native gateway) but lacks almost all implementation detail. The three steps read more like acceptance criteria than an actionable plan — there are no file paths, no code changes, no migration strategy, and no fallback path. As written, it would pass a goal-alignment check but would not give an implementer enough to start work without significant additional discovery.

## Strengths

- Correctly identifies the core problem: UI serving is still coupled to the legacy bootstrap layer while the rest of control delivery has moved to the native gateway
- Keeps scope bounded — doesn't try to rewrite the UI itself, just move serving ownership
- Explicitly calls out compatibility preservation as a verification target
- Aligns with the broader native-delivery denominator tracking (`2/8`), maintaining milestone coherence

## Concerns

- **HIGH** — No identification of the actual files or modules involved. Where does legacy UI serving currently live? Which gateway module will own it? Without file paths (e.g., references to `crates/gateway/src/`, the legacy bootstrap in `start.rs`, or static asset configuration), the plan is not executable.
- **HIGH** — No migration or cutover strategy. Will legacy and native paths coexist temporarily? Is there a feature flag? A config toggle? What happens if the native path fails — does the operator lose the UI entirely?
- **MEDIUM** — "Define native ownership" and "Define how UI actions align" are discovery tasks, not implementation steps. A plan should already contain the results of that discovery, or at minimum specify where the answers will be recorded.
- **MEDIUM** — No mention of how static assets (HTML/JS/CSS) are currently bundled, served, or cached. If the UI uses embedded assets (`include_bytes!`, `rust-embed`, or a build step), the serving transfer has build-system implications that aren't addressed.
- **MEDIUM** — No verification steps beyond a denominator check. How do you confirm the UI actually loads, that WebSocket/SSE live wiring still works, and that control routes respond correctly through the new path?
- **LOW** — The denominator advancement to `2/8` is stated as a verification target but there's no explanation of what `1/8` was or what the remaining `6/8` cover, making it hard to assess whether this phase is correctly scoped within the broader native-delivery arc.

## Suggestions

- **Add a file inventory**: List the current UI serving entry point (likely in `crates/cli/src/commands/start.rs` or the legacy gateway bootstrap), the target location in `crates/gateway/`, and any static-asset embedding or build artifacts involved.
- **Define the cutover mechanism**: Specify whether legacy serving is removed, gated behind a flag, or kept as a fallback. Given the project's emphasis on compatibility, a brief dual-serve window with a config toggle would be prudent.
- **Replace "Define" steps with concrete implementation steps**: e.g., "Add a `/ui` or `/control` route family in `openrustclaw-gateway` that serves the Control UI assets" and "Remove or redirect the equivalent route from the legacy bootstrap."
- **Add a real verification checklist**: At minimum — UI loads in browser via native gateway, WebSocket/live wiring connects, control actions (e.g., runtime inspect, memory view) round-trip correctly, legacy path is either removed or explicitly deprecated.
- **Document the asset-serving mechanism**: Specify whether assets are embedded at compile time, served from a directory, or proxied — this affects both the implementation and the deployment story.

## Risk Assessment

**MEDIUM-HIGH** — The goal is sound and well-scoped, but the plan as written is too abstract to execute safely. The primary risk is not that the wrong thing gets built, but that an implementer will have to do significant discovery mid-execution, potentially making ad-hoc decisions about fallback behavior, asset serving, and route wiring that should have been decided upfront. For a project that emphasizes "truthful" delivery and compatibility preservation, this plan needs one more round of concrete specification before it's ready for implementation.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed on the phase direction and sequencing, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for plan-specific concerns.

### Divergent Views
- Reviewers disagreed on overall risk level: gemini=HIGH, claude=MEDIUM-HIGH.
