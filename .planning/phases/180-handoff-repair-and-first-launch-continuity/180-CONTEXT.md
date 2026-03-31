# Phase 180: Handoff, Repair, and First-Launch Continuity - Context

**Gathered:** 2026-03-30
**Status:** Ready for planning
**Mode:** Smart discuss

<domain>
## Phase Boundary

Phase 180 should make the setup handoff, resume or repair prompts, and the first assistant launch remain aligned with the provider, access mode, and primary model captured during onboarding. It should close the milestone without inventing a separate launch configuration path.

</domain>

<decisions>
## Implementation Decisions

### Use setup-state next actions to distinguish model-lane substeps
Resume and repair do not need a brand-new onboarding step. They need more truthful `next_action` text for the existing model step so the operator can tell whether they still need provider selection, provider verification repair, or explicit model selection.

### Make handoff detail reflect the selected model lane
The setup handoff detail should include the selected provider, access mode, and primary model when available so the handoff surface reads like the actual onboarding result rather than a generic readiness sentence.

### Pass the selected model into first launch explicitly
The first assistant launch path should use the selected primary model directly when onboarding has it in memory, and otherwise fall back to the persisted runtime config model for the chosen provider.

</decisions>

<code_context>
## Existing Code Insights

- `crates/cli/src/commands/onboard.rs` already gates post-onboarding launch through `maybe_launch_assistant(...)`.
- `crates/cli/src/commands/chat.rs` already accepts an optional explicit model override in `load_chat_config(...)` and `chat::run(...)`.
- `crates/app/src/setup_lifecycle.rs` already computes handoff status and detail from setup state plus bootstrap outcomes.
- Setup handoff and inspect reports already carry the newly added selected primary model fields from Phase 179.

</code_context>

<specifics>
## Specific Ideas

- Add model-aware `next_action` generation for the onboarding model step.
- Include provider/access-model lane context in ready and verification-related handoff details.
- Pass the selected model into `chat::run(...)` during the optional first assistant launch.

</specifics>

<deferred>
## Deferred Ideas

- Richer UI presentation or formatting of the handoff report is outside the milestone boundary.
- Broader session-routing changes beyond honoring the selected onboarding model are out of scope.

</deferred>
