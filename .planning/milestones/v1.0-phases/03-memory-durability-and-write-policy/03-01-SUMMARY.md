---
phase: 03-memory-durability-and-write-policy
plan: 01
subsystem: assistant-memory-write-policy
tags:
  - memory
  - assistant
  - policy
  - tooling
provides:
  - Explicit assistant memory-write policy boundary
  - Structured policy metadata on stored memories
  - Regression coverage for allowed and blocked writes
affects:
  - Default assistant memory_store behavior
  - Prompt guidance for built-in memory usage
  - Shared memory policy primitives
tech-stack:
  added: []
  patterns:
    - Enforce assistant memory trust at the tool boundary, not only in prompt wording
key-files:
  created:
    - tests/integration/src/memory_policy_test.rs
  modified:
    - crates/memory/src/policies.rs
    - crates/memory/src/lib.rs
    - crates/agent/src/memory_tools.rs
    - crates/agent/src/prompt.rs
    - tests/integration/src/lib.rs
key-decisions:
  - The built-in assistant memory lane only stores explicit remember requests or obviously durable user or project facts
  - Ephemeral context and agent inference are blocked by default instead of being softly discouraged
patterns-established:
  - Stored assistant memories must carry machine-readable policy basis and reason metadata
duration: 45min
completed: 2026-03-26
---

# Phase 3: Memory Durability and Write Policy Summary

**Locked the default assistant memory lane behind an explicit policy gate so it no longer persists arbitrary conversation details.**

## Performance
- **Duration:** ~45 min
- **Tasks:** 3 completed
- **Files modified:** 6

## Accomplishments
- Added shared assistant write-policy primitives that classify explicit user requests, durable user facts, ephemeral context, and blocked agent inference.
- Tightened `memory_store` so tool calls must declare `basis` and `reason`, and blocked writes now return a non-error refusal instead of persisting anyway.
- Stored successful writes with `assistant_write_policy` metadata and added regression coverage for allowed and blocked cases.

## Task Commits
1. **Task 1: Enforce explicit assistant memory-write policy** - pending commit in current checkpoint

## Files Created/Modified
- `crates/memory/src/policies.rs` - Added assistant memory write-policy decision logic and tests
- `crates/memory/src/lib.rs` - Re-exported assistant write-policy types
- `crates/agent/src/memory_tools.rs` - Required policy basis and reason, enforced write gating, and persisted policy metadata
- `crates/agent/src/prompt.rs` - Tightened prompt instructions for memory tool usage
- `tests/integration/src/lib.rs` - Registered memory policy integration coverage
- `tests/integration/src/memory_policy_test.rs` - Added allowed and blocked assistant memory write tests

## Decisions & Deviations
This slice reused `RecallMemory::prepare_entry` and `MemoryPolicies` instead of creating a second tool-local memory shaping path. That keeps durable-memory rules centralized and prevents the prompt/tool contract from drifting away from the storage layer.

## Verification
- `cargo test -p openrustclaw-memory explicit_user_request_is_allowed -- --nocapture`
- `cargo test -p openrustclaw-agent prompt_describes_strict_memory_store_boundary -- --nocapture`
- `cargo test -p openrustclaw-integration-tests memory_policy -- --nocapture`

## Next Phase Readiness
Policy enforcement is now in place. The next work is to make those decisions legible from operator-facing CLI and Control UI inspection surfaces.
