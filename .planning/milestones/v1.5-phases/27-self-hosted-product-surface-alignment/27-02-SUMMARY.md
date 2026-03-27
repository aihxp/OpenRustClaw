# Plan 27-02 Summary: Tighten Self-Hosted Control Surface Copy

## What Changed

- Added clarifying copy above the `Self-Hosted Product Mode` panel in `/control/ui` so the dashboard states that it is the current deployment-path and transition-history surface.
- Tightened the default result text for mode transitions so the panel reads more like the same self-hosted product story as the docs.
- Expanded the static dashboard test to assert the self-hosted open-source wording remains present.

## Why It Matters

Phase 27 is about surface alignment, not new behavior. This small shipped-dashboard wording pass keeps the operator-facing UI in sync with the newly aligned public docs and preserves test coverage around that surface.

## Verification Notes

- `cargo test -p openrustclaw-cli dashboard_includes_self_hosted_product_mode_panel -- --nocapture`
