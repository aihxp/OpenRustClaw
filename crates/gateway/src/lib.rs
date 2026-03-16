//! WebSocket gateway and session routing for OpenRustClaw.

pub mod auth;
pub mod rate_limit;
pub mod router;
pub mod server;
pub mod sessions;

pub use server::GatewayServer;
pub use sessions::SessionManager;
