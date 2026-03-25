//! Shared utilities for the OpenRustClaw E2E crate.
//!
//! The concrete workflow tests live beside this module under `src/test_*.rs`.
//! Run the crate with `cargo test -p openrustclaw-e2e-tests --quiet`.

pub mod common;

// Re-export commonly used items
pub use common::*;
