//! Application-layer services for OpenRustClaw.
//!
//! This crate hosts use-case services and stable interfaces that delivery and
//! operator surfaces can call without reintroducing CLI or transport coupling.

pub mod agent_backend_catalog;
pub mod agent_backend_control;
pub mod agent_fabric_registry;
pub mod agent_route_policy;
pub mod assistant_continuity;
pub mod autonomy_lessons_control;
pub mod browser_backend_control;
pub mod browser_workflow_service;
pub mod channel_health_monitor;
pub mod channel_registry_lifecycle;
pub mod channel_routing;
pub mod compiled_skill_mcp;
pub mod compiled_skill_overview;
pub mod control_config;
pub mod control_diagnostics;
pub mod control_registry;
pub mod enterprise_access_control;
pub mod enterprise_admin;
pub mod greenfield_progress;
pub mod learning_review;
pub mod media_support;
pub mod memory_views;
pub mod mobile_operator;
pub mod mobile_runtime_control;
pub mod mobile_runtime_status;
pub mod onboarding_lane_catalog;
pub mod operator_status_control;
pub mod orchestration_reporting;
pub mod orchestration_routing;
pub mod runtime_maintenance_control;
pub mod runtime_maintenance_planning;
pub mod runtime_provider_switch;
pub mod runtime_reload_planning;
pub mod runtime_vault;
pub mod runtime_vault_control;
pub mod schedule_planning;
pub mod self_hosted_product;
pub mod setup_handoff;
pub mod setup_lifecycle;
pub mod skill_auth_plugin_binding;
pub mod skill_channel_extension_lifecycle;
pub mod skill_control;
pub mod skill_proposals;
pub mod skill_registry_mutation;
pub mod skill_voice_channel_control;
pub mod skill_voice_plugin_binding;
pub mod tool_execution_audit;
pub mod tool_host_service;
pub mod voice_call_reporting;
pub mod voice_runtime_lifecycle;
pub mod voice_runtime_reporting;
