# Summary 49-01: Locked the First Public Crate Boundary

## What Changed

- Chose `openrustclaw-core` as the first public crates.io target for v1.11 because it has no internal path-crate dependencies and already has a crate-local README.
- Updated the workspace repository URL to the live `aihxp/OpenRustClaw` repo.
- Added crates.io-facing metadata to `openrustclaw-core`: `readme`, `repository`, `homepage`, `documentation`, `keywords`, and `categories`.

## Result

The first publish boundary is now truthful: v1.11 does not pretend the whole workspace is ready for crates.io. Instead, it starts with `openrustclaw-core` and leaves the rest of the workspace out of the first public publish wave until later phases define a broader contract.

## Follow-on

- Phase 50 will harden the docs.rs rendering contract for `openrustclaw-core`.
- Phase 51 will define and verify the actual crates.io publish path.
