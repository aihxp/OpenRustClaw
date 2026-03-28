//! Greenfield application-layer services for OpenRustClaw.
//!
//! This crate is the first clean application shell introduced during the
//! brownfield-to-greenfield transition. It hosts use-case services and stable
//! interfaces that adapters can call without reintroducing CLI or transport
//! coupling.

pub mod setup_handoff;
