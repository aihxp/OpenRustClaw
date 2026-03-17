//! Horizontal E2E Tests - Full User Journeys
//!
//! These tests simulate complete user workflows across all subsystems:
//! - UI/API → Gateway → Agent → LLM → Tools → Database
//!
//! Each test verifies the entire flow works correctly end-to-end.

pub mod test_chat_journey;
// TODO: Add these test modules when implemented:
// pub mod test_agent_workflow;
// pub mod test_mcp_integration;
// pub mod test_memory_context;
