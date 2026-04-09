---
status: all_fixed
findings_in_scope: 1
fixed: 1
skipped: 0
iteration: 1
---

# Phase 18 Code Review Fix

Applied a manual fix for the Phase 18 intervention-history attribution gap.

## Outcome

- `WR-01` was fixed in `crates/cli/src/commands/orchestrate.rs`,
  `crates/cli/src/commands/start.rs`, `crates/cli/src/commands/enterprise_autonomy.rs`, and
  `crates/cli/src/main.rs`.

## Fix Summary

- Pause, resume, and kill now accept `ActiveRunInterventionRequest`, matching the attribution path
  already used by escalate and rollback.
- Runtime control handlers now thread the authenticated enterprise operator ID into those actions
  and record operator-tool results consistently for all five lifecycle controls.
- The enterprise kill-switch path now preserves the triggering operator ID when it terminates
  matching active runs.
- Native CLI pause, resume, and kill entry points were updated to use the new request shape without
  changing their behavior.
- Added a regression test proving pause, resume, and kill decisions retain the authenticated
  operator ID in durable intervention history.

## Verification

- `cargo fmt --all`
- `cargo test -p openrustclaw-cli pause_resume_and_kill_record_authenticated_operator_ids -- --nocapture`
- `cargo test -p openrustclaw-cli lifecycle_actions_record_decisions_and_rolled_back_state -- --nocapture`
- `cargo test -p openrustclaw-cli kill_switch_restores_baseline_and_marks_matching_runs -- --nocapture`
