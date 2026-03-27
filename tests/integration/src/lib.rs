//! Integration tests for OpenRustClaw.

pub mod common;

#[cfg(test)]
mod agent_runtime_test;
#[cfg(test)]
mod assistant_continuity_test;
#[cfg(test)]
mod browser_workflow_history_test;
#[cfg(test)]
mod channel_fixture_test;
#[cfg(test)]
mod coding_artifact_audit_test;
#[cfg(test)]
mod communications_audit_test;
#[cfg(test)]
mod documented_scenario_test;
#[cfg(test)]
mod fixture_suite_test;
#[cfg(test)]
mod gateway_test;
#[cfg(test)]
mod mcp_test;
#[cfg(test)]
mod memory_policy_test;
#[cfg(test)]
mod memory_workflow_test;
#[cfg(test)]
mod mobile_operator_report_test;
#[cfg(test)]
mod onboarding_test;
#[cfg(test)]
mod provider_chain_test;
#[cfg(test)]
mod runtime_operator_ops_test;
#[cfg(test)]
mod scheduler_test;
#[cfg(test)]
mod security_posture_test;
#[cfg(test)]
mod security_test;
#[cfg(test)]
mod tool_execution_history_test;
mod voice_operator_report_test;
#[cfg(test)]
mod voice_outcomes_test;
