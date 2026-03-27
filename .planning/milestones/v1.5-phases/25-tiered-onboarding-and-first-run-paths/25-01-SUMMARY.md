# Summary 25-01: Branch Onboarding By Deployment Path

- Updated [onboard.rs](/home/hprincivil/projects/OpenRustClaw/crates/cli/src/commands/onboard.rs) so the wizard now asks which self-hosted deployment path the operator wants: solo, multi-user team, company, or enterprise.
- The selected path now persists through [self_hosted.rs](/home/hprincivil/projects/OpenRustClaw/crates/cli/src/commands/self_hosted.rs) instead of staying as transient wizard-only state.
- Onboarding now chooses stronger defaults for company and enterprise paths while keeping lighter defaults for solo and team paths.
- Runtime-mode selection now starts from the deployment-path recommendation instead of always defaulting to `solo_claw`.

Requirements completed: `ONBR-01`, `ONBR-02`
