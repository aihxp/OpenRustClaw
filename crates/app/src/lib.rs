//! Greenfield application-layer services for OpenRustClaw.
//!
//! This crate is the first clean application shell introduced during the
//! brownfield-to-greenfield transition. It hosts use-case services and stable
//! interfaces that adapters can call without reintroducing CLI or transport
//! coupling.

pub mod autonomy_lessons_control;
pub mod browser_backend_control;
pub mod browser_workflow_service;
pub mod channel_registry_lifecycle;
pub mod compiled_skill_overview;
pub mod control_config;
pub mod control_diagnostics;
pub mod enterprise_access_control;
pub mod enterprise_admin;
pub mod greenfield_progress;
pub mod mobile_operator;
pub mod mobile_runtime_control;
pub mod mobile_runtime_status;
pub mod operator_status_control;
pub mod orchestration_reporting;
pub mod orchestration_routing;
pub mod runtime_maintenance_control;
pub mod runtime_maintenance_planning;
pub mod runtime_provider_switch;
pub mod runtime_reload_planning;
pub mod runtime_vault;
pub mod runtime_vault_control;
pub mod self_hosted_product;
pub mod setup_handoff;
pub mod skill_auth_plugin_binding;
pub mod skill_channel_extension_lifecycle;
pub mod skill_control;
pub mod skill_registry_mutation;
pub mod skill_voice_channel_control;
pub mod skill_voice_plugin_binding;
pub mod voice_runtime_lifecycle;
pub mod voice_runtime_reporting;
