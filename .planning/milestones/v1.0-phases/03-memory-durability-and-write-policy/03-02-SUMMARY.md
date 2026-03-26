---
phase: 03-memory-durability-and-write-policy
plan: 02
subsystem: memory-policy-inspection
tags:
  - memory
  - cli
  - control-ui
  - operator
provides:
  - CLI memory inspection with inline write-policy summaries
  - Control UI memory timeline with policy basis and reason
  - Regression coverage for memory policy rendering hooks
affects:
  - `openrustclaw memory timeline`
  - `openrustclaw memory search`
  - Control UI memory timeline
tech-stack:
  added: []
  patterns:
    - Reuse stored metadata directly in operator surfaces instead of inventing separate server-side heuristics
key-files:
  created: []
  modified:
    - crates/cli/src/commands/memory.rs
    - crates/cli/src/commands/control_ui.html
    - crates/cli/src/commands/control_ui.rs
key-decisions:
  - The existing control memory API already serialized full `MemoryEntry` metadata, so no `start.rs` change was needed for policy inspection
  - Operator surfaces should show concise `basis` and `reason` strings before forcing raw JSON inspection
patterns-established:
  - Assistant memory trust metadata is a shared contract across storage, CLI inspection, and browser operator UI
duration: 25min
completed: 2026-03-26
---

# Phase 3: Memory Durability and Write Policy Summary

**Made assistant memory-write decisions visible from shipped operator surfaces instead of hiding them in SQLite metadata.**

## Performance
- **Duration:** ~25 min
- **Tasks:** 2 completed
- **Files modified:** 3

## Accomplishments
- Updated `openrustclaw memory timeline` and `openrustclaw memory search` to append assistant write-policy summaries inline with each memory entry.
- Added Control UI memory timeline rendering for `basis` and `reason` so operators can distinguish explicit-request memories from durable-fact memories at a glance.
- Added lightweight regression coverage for the CLI summary formatter and the Control UI memory timeline wiring.

## Task Commits
1. **Task 1: Expose memory write-policy metadata in CLI and Control UI** - pending commit in current checkpoint

## Files Created/Modified
- `crates/cli/src/commands/memory.rs` - Added assistant write-policy summary formatting for timeline and search output
- `crates/cli/src/commands/control_ui.html` - Rendered policy basis and reason in the memory timeline table
- `crates/cli/src/commands/control_ui.rs` - Added a dashboard regression test for the memory policy timeline

## Decisions & Deviations
The plan originally listed `start.rs`, but the existing control endpoint already returned full memory metadata through `inspect::memory_timeline`. The implementation stayed narrower and consumed that existing contract directly from the browser UI.

## Verification
- `cargo test -p openrustclaw-cli assistant_write_policy_summary -- --nocapture`
- `cargo test -p openrustclaw-cli dashboard_includes_memory_policy_timeline_rendering -- --nocapture`

## Next Phase Readiness
Operator surfaces can now explain why a memory exists. The remaining work is to align docs and end-to-end verification with the stricter memory contract.
