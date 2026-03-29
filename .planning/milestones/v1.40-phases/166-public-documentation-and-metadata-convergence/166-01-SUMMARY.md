# Summary 166-01: Public documentation and metadata convergence

Public-facing documentation is now converged around product language instead of internal migration terminology. The old public architecture page was replaced by `application-boundaries.md`, mdBook navigation and related docs were updated to point at the new surface, and `crates/app/Cargo.toml` now describes the crate as application-layer services for OpenRustClaw instead of using migration language.

The public release story is also more coherent. Release-oriented docs now describe the active crates.io lane and current package flow plainly, which keeps the public docs aligned with the shipped product instead of older milestone memory.
