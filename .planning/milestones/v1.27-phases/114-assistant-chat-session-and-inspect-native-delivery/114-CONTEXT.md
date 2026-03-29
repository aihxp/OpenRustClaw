# Phase 114: Assistant, Chat, Session, and Inspect Native Delivery - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Define the first core operator CLI delivery family over app ports for assistant, chat, session, and inspect entrypoints.

</domain>

<decisions>
## Implementation Decisions

- Group assistant, chat, session, and inspect because they are core operator entrypoints with clear app-port contracts already present.
- Separate CLI parsing and rendering from app-use orchestration so the first native operator slice does not recreate command-local business logic.
- Use this phase to prove native CLI family grouping before the larger operator surfaces in later milestones.

</decisions>
