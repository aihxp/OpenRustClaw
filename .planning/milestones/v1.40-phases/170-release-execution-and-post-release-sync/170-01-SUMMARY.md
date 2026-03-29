# Summary 170-01: Release execution and post-release sync

The `0.1.1` public release is shipped. `openrustclaw-core 0.1.1` is now published on crates.io, the new crates.io token was saved locally with `cargo login`, the repo release-alignment commit landed, and the public tag `v0.1.1` was pushed so the repo’s version line matches the shipped crate.

The only remaining GitHub-side gap is a Release object created through `gh`, which was not possible on this machine because the local GitHub token was invalid. The source-controlled version line, package line, and pushed public tag are aligned.
