# Phase 25: Tiered Onboarding and First-Run Paths - Context

**Gathered:** 2026-03-27
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 24 defined the durable product-mode contract. Phase 25 should make onboarding actually use that contract so first-run setup feels like a self-hosted product with meaningful paths instead of one flat wizard.

This phase should:

- let the operator choose solo, team, company, or enterprise during onboarding
- persist that choice into the product-mode contract during the wizard
- feed the chosen mode back into onboarding defaults, workspace status, and first-start diagnostics
- stop short of reversible upgrade or downgrade actions, which belong in the lifecycle phase

</domain>

<decisions>
## Implementation Decisions

### Keep the existing onboarding skeleton
The current wizard already has a good step structure. This phase should branch the entry path and defaults rather than rewrite the entire wizard flow.

### Use the product-mode contract as the single source of truth
Onboarding should write the selected deployment path through the new manifest instead of storing a second mode copy in ad hoc files.

### Make diagnostics informative before they become strict
Doctor should surface missing explicit product-mode selection, but it should not block first start the same way missing provider keys or missing onboarding state do.

</decisions>

<code_context>
## Existing Code Insights

- `crates/cli/src/commands/onboard.rs` already owns interactive setup, workspace-state detection, and runtime-mode defaults.
- `crates/cli/src/commands/doctor.rs` already performs post-onboarding readiness checks and can surface mode-selection drift.
- `crates/cli/src/commands/self_hosted.rs` now provides the contract that onboarding should write and inspect.

</code_context>

<specifics>
## Specific Ideas

- add an early deployment-path selector to the wizard
- default QuickStart vs Advanced based on the chosen mode
- default runtime-mode recommendations based on the selected deployment path
- show the selected product mode in workspace-status and completion output
- add a doctor warning when no explicit self-hosted mode has been selected yet

</specifics>

<deferred>
## Deferred Ideas

- runtime or UI actions for product-mode transitions after first run
- transition history or downgrade warnings
- docs alignment beyond the minimum operator-facing output in the wizard and doctor

</deferred>
