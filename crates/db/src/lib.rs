//! Database layer for OpenRustClaw.
//!
//! Uses sqlx (async, primary) for general persistence,
//! libSQL for vector operations, and rusqlite for sync CLI fallback.

pub mod migrate;
pub mod models;
pub mod pool;

pub use migrate::run_migrations;
pub use pool::init_pool;
pub use sqlx::SqlitePool;
