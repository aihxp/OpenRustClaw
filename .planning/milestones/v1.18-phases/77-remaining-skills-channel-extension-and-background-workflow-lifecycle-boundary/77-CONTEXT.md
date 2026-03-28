# Phase 77: Remaining Skills Channel-Extension and Background Workflow Lifecycle Boundary - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning

## Phase Boundary

Move the remaining ranked channel-extension and background workflow lifecycle seam out of `skills.rs` so the last seam in the current canonical inventory no longer deepens the legacy hotspot.

## Decisions

- The remaining ranked seam is the combination of background workflow scheduling and channel-extension binding because both depend on the same compiled background-service resolution rules.
- `skills.rs` should remain the adapter that loads compiled artifacts, opens the scheduler database, persists channel bindings, and publishes plugin events.
- The new application service should own trigger validation, component-presence validation, job-shaping, binding metadata shaping, and operator-facing result composition.

## Existing Code Insights

- `schedule_background_service_data` still validates trigger arguments, shapes durable scheduler job metadata, and returns the operator-facing scheduling report inline.
- `bind_channel_extension_data` still validates trigger semantics, shapes channel-binding metadata, and returns the operator-facing bind result inline.
- The shared seam is narrow enough to extract without reopening the existing read-only extension manifest and background-service inspection helpers.
