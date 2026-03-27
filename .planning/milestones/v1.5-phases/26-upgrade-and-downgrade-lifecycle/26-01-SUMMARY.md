# Summary 26-01: Add Durable Product-Mode Transition Receipts

- Extended [self_hosted.rs](/home/hprincivil/projects/OpenRustClaw/crates/cli/src/commands/self_hosted.rs) with explicit product-mode transition requests and a durable event ledger.
- Product-mode changes are now classified as `upgrade` or `downgrade`, not just silent overwrites.
- The product-mode summary in [inspect.rs](/home/hprincivil/projects/OpenRustClaw/crates/cli/src/commands/inspect.rs) now includes recent transitions plus current retained-state warnings.
- Downgrade warnings now call out retained enterprise access or autonomy state instead of hiding it.

Requirements completed: `LIFE-01`, `LIFE-02`
