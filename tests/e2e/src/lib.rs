//! End-to-End (E2E) tests for OpenRustClaw.
//!
//! This crate contains comprehensive end-to-end tests that verify the entire
//! system works together correctly. Tests cover:
//!
//! - Chat workflows with tool calling
//! - Memory storage, retrieval, and search
//! - Scheduler job execution and retries
//! - Security (auth, origin validation, rate limiting)
//! - Provider fallback and resilience
//!
//! ## Running Tests
//!
//! ### Mock-based tests (default):
//! ```bash
//! cargo test --test e2e
//! ```
//!
//! ### Live provider tests (requires API keys):
//! ```bash
//! export E2E_LIVE=1
//! export OPENAI_API_KEY=sk-...
//! export ANTHROPIC_API_KEY=sk-ant-...
//! cargo test --test e2e
//! ```
//!
//! ### Run specific test file:
//! ```bash
//! cargo test --test e2e test_chat_workflow
//! ```
//!
//! ### Run with output:
//! ```bash
//! cargo test --test e2e -- --nocapture
//! ```

pub mod common;

mod test_chat_workflow;
mod test_memory_workflow;
mod test_provider_fallback;
mod test_scheduler_workflow;
mod test_security_workflow;
