//! Vertical E2E Tests - Layer-Specific Testing
//!
//! These tests focus on individual layers in isolation:
//! - API Layer: HTTP endpoints, WebSocket, SSE
//! - DB Layer: SQLite operations, migrations, queries
//! - Provider Layer: LLM provider fallbacks, retries
//! - Gateway Layer: Rate limiting, auth, routing

pub mod test_api_layer;
pub mod test_db_layer;
pub mod test_provider_layer;
pub mod test_gateway_layer;
