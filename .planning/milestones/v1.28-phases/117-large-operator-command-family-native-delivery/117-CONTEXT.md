# Phase 117: Large Operator Command Family Native Delivery - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Define the native CLI delivery path for the remaining large operator-facing command families such as browser, orchestration, mobile, voice runtime, onboarding, skills, and self-hosted flows.

</domain>

<decisions>
## Implementation Decisions

- Group the remaining large operator command families into one native-delivery planning slice because they still dominate the legacy CLI footprint.
- Map those families to app ports explicitly instead of preserving command-file ownership as implicit architecture.
- Keep the milestone at the planning-contract level so later implementation can move incrementally by family without rediscovering the grouping.

</decisions>
