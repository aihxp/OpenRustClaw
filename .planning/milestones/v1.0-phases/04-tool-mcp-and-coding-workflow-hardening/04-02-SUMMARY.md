---
phase: 04-tool-mcp-and-coding-workflow-hardening
plan: 02
subsystem: cursor-coding-artifacts
tags:
  - coding
  - cursor
  - artifacts
  - audit
provides:
  - Durable Cursor coding-run artifact store under `.claw/control`
  - Artifact metadata attached to coding tool results
  - Diff-aware evidence for file edits in git-backed workspaces
affects:
  - Cursor ACP coding tools
  - Cursor CLI setup and status guidance
  - Coding auditability for workspace-bounded runs
tech-stack:
  added: []
  patterns:
    - Capture coding evidence at the shared tool-registry seam instead of duplicating audit logic in every individual Cursor tool
key-files:
  created: []
  modified:
    - crates/cursor/src/tools/mod.rs
    - crates/cursor/tests/integration_tests.rs
    - crates/cli/src/commands/cursor.rs
key-decisions:
  - Cursor coding artifacts should be best-effort and should not break the underlying tool execution path if artifact persistence fails
  - Mutating file tools should capture a git diff preview when a git-backed workspace makes that evidence available
patterns-established:
  - Coding tools should return stable artifact pointers so operators can review evidence after the run instead of trusting transient IDE context
duration: 40min
completed: 2026-03-26
---

# Phase 4: Tool, MCP, and Coding Workflow Hardening Summary

**Made the Cursor coding lane auditable by persisting per-run artifacts for command, edit, and other coding-tool executions under the workspace control plane.**

## Performance
- **Duration:** ~40 min
- **Tasks:** 3 completed
- **Files modified:** 3

## Accomplishments
- Added a durable Cursor coding artifact store under `.claw/control/cursor-tool-runs` with an index plus per-run JSON artifacts.
- Wrapped the shared Cursor tool registry so successful and failed coding-tool executions persist audit artifacts and successful outputs expose `_artifact` metadata with a stable path.
- Captured git diff previews for mutating file tools when the workspace is git-backed, so edit operations leave reviewable change evidence instead of only success booleans.
- Updated `openrustclaw cursor` output to point operators at the coding artifact root.

## Task Commits
1. **Task 1: Make coding workflows bounded and auditable** - pending commit in current checkpoint

## Files Created/Modified
- `crates/cursor/src/tools/mod.rs` - Added central coding artifact persistence and artifact loading helpers to the Cursor tool registry
- `crates/cursor/tests/integration_tests.rs` - Added command and edit artifact regression coverage
- `crates/cli/src/commands/cursor.rs` - Surfaced the coding artifact root in setup and status output

## Decisions & Deviations
This slice landed inside the shared Cursor tool registry rather than only the CLI wrapper because that is where all the real inspect, edit, run, git, and linter actions already converge. Centralizing there keeps the audit contract consistent across the whole coding lane.

## Verification
- `cargo test -p openrustclaw-cursor test_tool_execution_run_command -- --nocapture`
- `cargo test -p openrustclaw-cursor test_edit_file_emits_coding_artifact -- --nocapture`

## Next Phase Readiness
Phase 4 now has both a runtime tool ledger and a coding artifact store. The remaining work is to expose that audit path more clearly through operator surfaces and docs.
