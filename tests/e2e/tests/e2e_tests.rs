//! OpenRustClaw E2E Tests
//!
//! Run with: cargo test --test e2e_tests [smoke|horizontal|vertical|regression]

// Re-export all test modules
mod horizontal;
mod vertical;
mod smoke;
mod regression;

// The actual tests are in the submodules
// Each submodule imports common utilities via:
// use openrustclaw_e2e_tests::common::*;
