# Requirements: OpenRustClaw

## Active Milestone: v1.13 Brownfield-to-Greenfield Transition

**Goal:** Create a greenfield-style architecture lane inside the existing OpenRustClaw repo so future development lands on clean boundaries instead of accumulating more brownfield coupling.

## Requirements

### GF-01 Greenfield boundary contract

OpenRustClaw must define one canonical architecture contract that distinguishes the new greenfield lane from the remaining brownfield surfaces, including module ownership, dependency direction, no-touch legacy seams, and the first migration candidates.

**Acceptance signals:**
- the repo has one explicit statement of greenfield versus legacy boundaries
- dependency direction and ownership rules are documented clearly enough to enforce in review
- the first migration targets are selected instead of leaving the transition abstract

### GF-02 Clean core shell and service interfaces

The repo must gain a real greenfield core shell and stable service interface layer that future work can target without reaching directly into legacy command or runtime modules.

**Acceptance signals:**
- a new bounded core or application shell exists in shipped code
- critical interfaces are expressed through stable abstractions rather than ad hoc cross-module calls
- new work can enter through the new shell without depending on legacy internals by default

### GF-03 First migrated vertical slice

At least one high-value shipped vertical slice must be moved into the new architecture lane so the milestone proves migration with production code instead of architecture notes alone.

**Acceptance signals:**
- one real user or operator flow is served through the new boundary
- the migrated slice reduces direct coupling to brownfield modules measurably
- verification proves the migrated slice still behaves truthfully end to end

### GF-04 Contributor defaults and compatibility path

Contributor and operator guidance must make the new greenfield lane the default for future work, while preserving a bounded compatibility and deprecation path for the remaining brownfield surface.

**Acceptance signals:**
- docs explain where new code should live and what legacy seams are temporary only
- compatibility or adapter rules are explicit for mixed old and new surfaces
- deprecation or follow-up migration paths are preserved instead of implied

## Most Recent Archive

- Last shipped milestone: `v1.12 Secure Node Connectivity and SSH Tunnel Revisit`
- Archived requirements: `.planning/milestones/v1.12-REQUIREMENTS.md`
- Archived verification bundle: `.planning/milestones/v1.12-VERIFICATIONS.md`

## Next Step

Start execution with `$gsd-plan-phase 57` or `$gsd-autonomous`.
