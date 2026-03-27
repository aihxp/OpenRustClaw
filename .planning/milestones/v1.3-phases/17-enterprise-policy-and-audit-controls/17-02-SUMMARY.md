# 17-02 Summary

## What Landed

Added a durable enterprise audit export path:

- `POST /control/enterprise/audit/export` writes a JSON bundle to `.claw/control/enterprise/exports/`
- the bundle captures the current enterprise policy summary
- the bundle includes recent enterprise foundations evidence and recent operator tool history
- export is protected as an enterprise-scoped action instead of being left as an unaudited local script

## Key Files

- `crates/cli/src/commands/enterprise_policy.rs`
- `crates/cli/src/commands/enterprise_access.rs`
- `crates/cli/src/commands/start.rs`

## Verification

- `cargo test -p openrustclaw-cli enterprise_policy -- --nocapture`
- `cargo test -p openrustclaw-cli protected_scope_classifies_sensitive_routes -- --nocapture`
