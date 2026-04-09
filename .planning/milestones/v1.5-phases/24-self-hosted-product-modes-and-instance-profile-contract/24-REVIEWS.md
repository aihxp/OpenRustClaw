---
phase: 24
requested_reviewers: [gemini, claude]
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T17:24:36.130Z
plans_reviewed: [24-01-PLAN.md, 24-02-PLAN.md]
---
# Cross-AI Plan Review — Phase 24

## Gemini Review

Here is a structured review of the proposed implementation plans for Phase 24 of OpenRustClaw.

### 1. Summary

The proposed plans effectively break down the phase goals into two logical, sequential steps: defining the durable data contract (24-01) and surfacing that contract to the operator (24-02). The plans correctly align with the user decisions to treat the product mode as explicit data within the existing `.claw/control` directory and to prioritize the read path before tackling write-heavy lifecycle transitions. However, the plans are overly brief and lack technical specificity regarding the schema, backward compatibility for existing installations, and exact integration points with the existing control registry.

### 2. Strengths

*   **Clean Separation of Concerns:** Splitting the core data contract (24-01) from the inspection and UI surfacing (24-02) ensures a stable foundation before touching the presentation layer.
*   **Alignment with Constraints:** The plans explicitly respect the directive to separate product mode from runtime execution topology and accurately target the `.claw/control` storage root.
*   **Incremental Approach:** By deferring interactive upgrades, downgrades, and onboarding branching, the plans keep the scope tight and focused purely on establishing the truth of the state.
*   **Targeted Verification:** Both plans explicitly call out unit and integration testing for serialization, validation, and UI wiring, ensuring the contract is reliable for future phases.

### 3. Concerns

*   **HIGH: Missing Backward Compatibility Strategy.** The plans do not specify what happens when an existing OpenRustClaw v1.47 installation is loaded and the new `.claw/control/product_mode.json` (or similar manifest) does not exist. There must be a defined default fallback (presumably "solo") to prevent panics or degraded states on startup.
*   **MEDIUM: Undefined Schema and Metadata.** Plan 24-01 states it will "capture enough mode metadata" but fails to define what that metadata actually is. Without a clear schema (e.g., `mode_type`, `operator_footprint`, `recommended_runtime`), the implementation risks being incomplete or requiring immediate refactoring in the next phase.
*   **MEDIUM: Ambiguous Module Integration.** The context notes that `crates/cli/src/commands/control.rs` currently owns the `.claw/control` registry, but Plan 24-01 proposes creating `self_hosted.rs` without explaining how it integrates with or extends the existing `control.rs` logic.
*   **LOW: Vague API Routing.** Plan 24-02 mentions adding a route to `start.rs` but doesn't specify the route path or payload structure, which could lead to inconsistencies with existing `/control/...` API conventions.

### 4. Suggestions

*   **Define the Manifest Schema:** Update Plan 24-01 to explicitly define the Rust struct and serialization format (e.g., JSON) for the manifest. Include fields for `mode` (enum: Solo, Team, Company, Enterprise) and any required static metadata.
*   **Specify the Default/Fallback State:** Explicitly mandate in Plan 24-01 that the system must gracefully handle missing manifests by defaulting to the `Solo` profile for existing workspaces.
*   **Clarify File Ownership:** Ensure Plan 24-01 clearly defines whether `self_hosted.rs` acts as a sub-module of `control.rs` or a sibling, and how file I/O locks/reads are shared for the `.claw/control` directory.
*   **Formalize the API Contract:** In Plan 24-02, specify the exact endpoint (e.g., `GET /api/control/product-mode`) and the shape of the typed JSON response that the Control UI will consume.
*   **Add UI Fallback States:** Ensure Plan 24-02 includes testing for how the Control UI card renders if the API route fails or returns unexpected data.

### 5. Risk Assessment

**LOW**

The overall risk of these plans is Low. The scope is purely additive (creating a new manifest file and exposing read-only routes/UI components). Because this phase defers write-heavy lifecycle changes and does not alter the core execution or routing topology, a failure here is unlikely to break existing assistant functionality or corrupt existing data. Addressing the backward compatibility concern (defaulting to "solo" for legacy workspaces) will mitigate the only significant risk of startup failures.

---

## Claude Review

The review is complete. Here's the summary:

## Overall: LOW-MEDIUM Risk

Both plans are correctly scoped and sequenced, but underspecified for a phase whose purpose is establishing a **contract**.

**Key gap across both plans:** Neither defines the actual data shapes — the manifest struct or the JSON summary response. This is the main coordination risk between the two plans.

**Top recommendations:**
1. Pin the manifest struct (fields, types, serde format) in Plan 24-01 before coding
2. Specify the default mode and initialization path for new/existing installs
3. Define the JSON response shape so the Rust handler and HTML card can be reviewed together
4. Bound the `start.rs` changes to route registration only — it's a known hotspot
5. State the auth posture for the new `/control/self-hosted/product-mode` route

The phase boundary itself is well-drawn. The plans just need more structural detail to match the ambition of shipping a durable contract.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed on the phase direction and sequencing, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for plan-specific concerns.

### Divergent Views
- Not all reviewers exposed a phase-level risk label; parsed labels: gemini=LOW.
