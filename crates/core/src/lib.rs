//! Shared foundations for OpenRustClaw crates.
//!
//! `openrustclaw-core` is the lowest-level public crate in the workspace. It
//! defines the shared types, traits, configuration structures, and error
//! surfaces used by the rest of the platform.
//!
//! # Modules
//!
//! - [`types`] contains the shared domain models for messages, sessions, tool
//!   calls, completion requests, and other runtime data.
//! - [`traits`] defines the core subsystem contracts for providers, tools,
//!   channels, and memory stores.
//! - [`config`] contains serializable application configuration structures.
//! - [`error`] defines the common error enums used across OpenRustClaw crates.
//!
//! # Quick start
//!
//! ```rust
//! use openrustclaw_core::types::{Message, Platform, Session};
//!
//! let message = Message::user("Hello from crates.io");
//! let session = Session::new_dm("user-123", Platform::Cli);
//!
//! assert_eq!(message.content, "Hello from crates.io");
//! assert_eq!(session.user_id, "user-123");
//! ```

pub mod config;
pub mod error;
pub mod traits;
pub mod types;
