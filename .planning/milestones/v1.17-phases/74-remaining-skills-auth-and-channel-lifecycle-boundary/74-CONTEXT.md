# Phase 74: Remaining Skills Auth and Channel Lifecycle Boundary - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning

## Phase Boundary

Move one real auth-plugin or channel-extension lifecycle lane out of `skills.rs` so the remaining skills hotspot keeps shrinking after the compiled-skill, install/update/uninstall, and voice-plugin binding extractions.

## Decisions

- The auth-plugin bind lane is the cleanest remaining lifecycle slice because it already has bounded validation, default-key derivation, registry persistence, and event publication.
- The OIDC authorization and token exchange flow should remain out of scope for this phase because it would broaden the slice into session and vault mutation behavior.
- `skills.rs` should remain the adapter around compiled-skill loading, background-service resolution, registry writes, and plugin-event publication.

## Existing Code Insights

- `bind_auth_plugin_data` currently owns both policy validation and binding composition inline.
- `authorize_auth_plugin_data` and `exchange_auth_plugin_data` depend on the persisted binding but can remain on the existing adapter lane for now.
- The remaining channel-extension/background workflow lane is still queued after auth-plugin binding.
