# Crates.io Release

OpenRustClaw's current public Rust package surface is intentionally narrow. The maintained crates.io lane is:

- `openrustclaw-core`

The rest of the workspace remains out of the public crates.io lane until it has an explicit publish contract.

This semver package lane is different from planning milestone tags such as `v1.45`. Milestone tags and milestone GitHub releases are archive markers for shipped planning slices. The public package line is the semver `1.4.x` line for `openrustclaw-core`.

The current shipped public package baseline has advanced through `1.4.9`. The `v1.4.1` through `v1.4.9` tags should therefore be read as shipped semver releases that the planning deck later captured under `v1.46`, not as a second milestone numbering system.

## Preflight

Before attempting a publish:

1. Ensure you have a valid crates.io token available through `cargo login` or `CARGO_REGISTRY_TOKEN`.
2. Verify the crate metadata, package contents, and docs.rs build path:

```bash
bash scripts/check-crates-io-readiness.sh openrustclaw-core
```

3. Confirm the crate version in `crates/core/Cargo.toml` resolves to the intended release version through the workspace version.
4. Confirm the current published version on crates.io so the next release version is an increment rather than a duplicate.

## Publish Order

For the current package release lane, publish only:

1. `openrustclaw-core`

No other workspace crate is part of the public crates.io contract yet.

## Publish

With credentials configured:

```bash
cargo publish -p openrustclaw-core
```

## Post-Publish Checks

After publish succeeds:

1. Confirm the crate appears on crates.io with the new version.
2. Confirm the docs.rs build starts for `openrustclaw-core`.
3. Check the docs.rs landing page once the build finishes.
4. Record the crate version, crates.io URL, and docs.rs URL in milestone verification.

## Current Blocker Shape

If no crates.io credential is configured, or if the configured token lacks publish permission, stop before publish and preserve the dry-run evidence plus the explicit error. That is a truthful operator checkpoint; do not claim public publication until crates.io and docs.rs are both visible.
