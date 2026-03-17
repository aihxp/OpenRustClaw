//! OpenRustClaw E2E Testing Framework - Common Utilities
//!
//! This crate provides shared utilities for E2E tests.
//! The actual tests are in the `tests/` directory.
//!
//! ## Running Tests
//!
//! ```bash
//! # Smoke tests (fast)
//! cargo test --test e2e_tests smoke
//!
//! # Horizontal tests (user journeys)
//! cargo test --test e2e_tests horizontal
//!
//! # Vertical tests (layer-specific)
//! cargo test --test e2e_tests vertical
//!
//! # Regression tests (full suite)
//! cargo test --test e2e_tests regression
//! ```

pub mod common;

// Re-export commonly used items
pub use common::*;
