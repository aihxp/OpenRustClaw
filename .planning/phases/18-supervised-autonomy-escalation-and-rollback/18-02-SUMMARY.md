# 18-02 Summary

## What Landed

Exposed supervised lifecycle controls and reports through the runtime control plane and existing operator UI.

- new escalation and rollback control routes for active orchestration runs
- enterprise route protection now covers supervised lifecycle actions
- active supervision payloads include lifecycle and decision history
- `/control/ui` now surfaces lifecycle state, intervention history, and escalation or rollback actions in the active orchestration panel

## Key Files

- `crates/cli/src/commands/start.rs`
- `crates/cli/src/commands/enterprise_access.rs`
- `crates/cli/src/commands/control_ui.html`
- `crates/cli/src/commands/control_ui.rs`

## Verification

- `cargo test -p openrustclaw-cli read_active_run_supervision_includes_recent_events_and_attention_signals -- --nocapture`
- `cargo test -p openrustclaw-cli protected_scope_classifies_sensitive_routes -- --nocapture`
- `cargo test -p openrustclaw-cli dashboard_includes_orchestration_supervision_tables -- --nocapture`
