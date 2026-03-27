# Summary 25-02: Reflect Deployment Path In First-Start Diagnostics

- [workspace_status](/home/hprincivil/projects/OpenRustClaw/crates/cli/src/commands/onboard.rs) now includes explicit self-hosted product-mode presence and label reporting.
- [doctor.rs](/home/hprincivil/projects/OpenRustClaw/crates/cli/src/commands/doctor.rs) now warns when no explicit self-hosted product mode has been selected yet.
- First-start readiness remains honest: missing product mode is visible, but it does not block startup the way missing provider credentials or missing onboarding-managed state do.

Requirements completed: `ONBR-01`, `ONBR-02`
