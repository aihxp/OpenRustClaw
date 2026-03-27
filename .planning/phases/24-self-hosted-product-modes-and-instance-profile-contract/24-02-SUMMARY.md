# Summary 24-02: Surface The Product Mode Through Shipped Inspection

- Added [self_hosted_product_mode_summary](/home/hprincivil/projects/OpenRustClaw/crates/cli/src/commands/inspect.rs) so the product-mode contract is available as typed runtime inspection instead of doc-only copy.
- Exposed the summary at `/control/self-hosted/product-mode` in [start.rs](/home/hprincivil/projects/OpenRustClaw/crates/cli/src/commands/start.rs).
- Added a `Self-Hosted Product Mode` summary card to [control_ui.html](/home/hprincivil/projects/OpenRustClaw/crates/cli/src/commands/control_ui.html) and locked the wiring with a static test in [control_ui.rs](/home/hprincivil/projects/OpenRustClaw/crates/cli/src/commands/control_ui.rs).
- The shipped surface now makes the self-hosted and open-source identity explicit, while also showing onboarding path, operator footprint, runtime recommendation, and valid next modes.

Requirements completed: `MODE-01`, `MODE-02`
