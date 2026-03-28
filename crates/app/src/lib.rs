//! Greenfield application-layer services for OpenRustClaw.
//!
//! This crate is the first clean application shell introduced during the
//! brownfield-to-greenfield transition. It hosts use-case services and stable
//! interfaces that adapters can call without reintroducing CLI or transport
//! coupling.

pub mod compiled_skill_overview;
pub mod enterprise_access_control;
pub mod enterprise_admin;
pub mod mobile_operator;
pub mod runtime_provider_switch;
pub mod runtime_reload_planning;
pub mod runtime_vault;
pub mod runtime_vault_control;
pub mod self_hosted_product;
pub mod setup_handoff;
pub mod skill_registry_mutation;
pub mod skill_voice_plugin_binding;
