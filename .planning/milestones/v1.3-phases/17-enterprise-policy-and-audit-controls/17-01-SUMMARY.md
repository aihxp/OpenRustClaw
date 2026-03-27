# 17-01 Summary

## What Landed

Added a unified enterprise policy surface in the runtime control plane:

- new `enterprise_policy.rs` module for durable enterprise policy state
- `GET /control/enterprise/policy` for typed policy inspection
- `PUT /control/enterprise/policy` for bounded policy updates
- runtime approval-policy updates now write through the control runtime manifest
- browser backend allowlist and wrapper/cloud policy updates now write through runtime config
- mobile command default approval behavior now honors enterprise policy overrides instead of only hardcoded command defaults

## Key Files

- `crates/cli/src/commands/enterprise_policy.rs`
- `crates/cli/src/commands/control.rs`
- `crates/cli/src/commands/mobile.rs`
- `crates/cli/src/commands/start.rs`
- `crates/cli/src/commands/enterprise_access.rs`

## Verification

- `cargo test -p openrustclaw-cli enterprise_policy -- --nocapture`
- `cargo test -p openrustclaw-cli dispatch_command_uses_enterprise_mobile_policy_override -- --nocapture`
