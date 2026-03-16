//! Model Context Protocol (MCP) client and server for OpenRustClaw.
//!
//! OpenRustClaw acts as both an MCP client (connect to external servers)
//! and an MCP server (expose tools to Claude Desktop/Code/Cursor).

pub mod client;
pub mod registry;
pub mod server;
pub mod translate;
pub mod transport;

pub use client::McpClient;
pub use registry::McpRegistry;
pub use server::McpServer;
