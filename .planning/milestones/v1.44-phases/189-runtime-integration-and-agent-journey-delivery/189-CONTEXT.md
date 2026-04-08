# Phase 189: Runtime Integration and Agent Journey Delivery - Context

**Gathered:** 2026-04-08
**Status:** Ready for execution

<domain>
## Phase Boundary

Turn delegated local-agent lanes from passive discovery metadata into bounded runtime execution paths. This phase must let eligible work run through installed local agent CLIs with policy checks, audit receipts, and control-registry integration, without weakening approval or autonomy boundaries.

</domain>

<decisions>
## Implementation Decisions

### Runtime boundary
- **D-01:** Delegated local-agent execution should plug into the existing provider factory and orchestration/runtime plumbing instead of creating a parallel runner that bypasses model-profile selection.
- **D-02:** Bounded delegated execution should default to the strictest supported non-interactive modes (`plan`, read-only sandbox, or equivalent) unless a later milestone widens that boundary deliberately.
- **D-03:** Delegated runtime receipts should reuse the existing external-backend audit log and tool-execution history surfaces so operators do not need a second inspection pane.

### Control boundary
- **D-04:** Control-registry model profiles may reference delegated backend ids directly when the local backend is execution-eligible.
- **D-05:** Vendor-managed delegated backends should avoid guessed model names; control init may seed templates with `vendor-managed` placeholders and fallback chains instead of pretending model catalogs are known.

</decisions>

<canonical_refs>
## Canonical References

- `.planning/PROJECT.md`
- `.planning/REQUIREMENTS.md`
- `.planning/ROADMAP.md`
- `.planning/STATE.md`
- `.planning/phases/188-onboarding-and-model-selection-cohesion/188-02-SUMMARY.md`
- `crates/cli/src/commands/runtime.rs`
- `crates/cli/src/commands/control.rs`
- `crates/cli/src/commands/inspect.rs`
- `crates/cli/src/commands/browser.rs`
- `crates/app/src/agent_backend_control.rs`
- `crates/app/src/browser_backend_control.rs`

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable assets
- `create_provider_from_config` is the canonical runtime provider factory used across health checks, start flows, and orchestration.
- External backend policy and audit contracts already exist and are shared between browser wrappers and delegated agent backend policy.
- Inspect already exposes external backend audit evidence and tool-execution history, so delegated runtime receipts can reuse those lanes.
- `control init` already seeds model profiles and runtime manifests, making it the right place to seed delegated backend templates.

### Current disconnects
- Delegated local-agent backends were visible in onboarding and inspect, but runtime execution still required direct provider SDKs only.
- Control-registry defaults did not expose delegated backend model profiles, which kept delegated runtime lanes hidden from normal operator setup.
- Audit evidence existed structurally, but delegated runtime work did not yet write receipts into those surfaces.

</code_context>

<deferred>
## Deferred Ideas

- Full streaming delegated backend support
- Per-run delegated session ids propagated through every orchestration stage
- Richer UI-first delegated backend inspection panels

</deferred>

---

*Phase: 189-runtime-integration-and-agent-journey-delivery*
*Context gathered: 2026-04-08*
