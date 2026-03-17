//! Security hardening layer for OpenRustClaw.
//!
//! Addresses documented OpenClaw gaps: CVE-2026-25253 (origin validation),
//! prompt injection defense, Ed25519 skill verification, and session isolation.

pub mod audit;
pub mod auth;
pub mod input_sanitizer;
pub mod isolation;
pub mod origin_check;
pub mod skill_verifier;
pub mod sso;

pub use auth::AuthManager;
pub use input_sanitizer::InputSanitizer;
pub use origin_check::OriginValidator;
pub use skill_verifier::SkillVerifier;
pub use sso::{SsoClient, SsoConfig, SsoError, SsoProvider, SsoRegistry, SsoTokens, SsoUserInfo};
