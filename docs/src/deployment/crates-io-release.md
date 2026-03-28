# Crates.io Release

OpenRustClaw's first public Rust package surface is intentionally narrow. For the `v1.11` release lane, the publish target is:

- `openrustclaw-core`

The rest of the workspace remains out of the first crates.io release wave until it has an explicit publish contract.

## Preflight

Before attempting a publish:

1. Ensure you have a valid crates.io token available through `cargo login` or `CARGO_REGISTRY_TOKEN`.
2. Verify the crate metadata, package contents, and docs.rs build path:

```bash
bash scripts/check-crates-io-readiness.sh openrustclaw-core
```

3. Confirm the crate version in `crates/core/Cargo.toml` resolves to the intended release version through the workspace version.

## Publish Order

For the first public package release, publish only:

1. `openrustclaw-core`

No other workspace crate is part of the first public crates.io contract yet.

## Publish

With credentials configured:

```bash
cargo publish -p openrustclaw-core
```

## Post-Publish Checks

After publish succeeds:

1. Confirm the crate appears on crates.io.
2. Confirm the docs.rs build starts for `openrustclaw-core`.
3. Check the docs.rs landing page once the build finishes.
4. Record the crate version, crates.io URL, and docs.rs URL in milestone verification.

## Current Blocker Shape

If no crates.io credential is configured, stop before publish and preserve the dry-run evidence. That is a truthful operator checkpoint; do not claim public publication until crates.io and docs.rs are both visible.
