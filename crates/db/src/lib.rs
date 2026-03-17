//! Database layer for OpenRustClaw.
//!
//! Uses sqlx (async, primary) for general persistence,
//! libSQL for vector operations, and rusqlite for sync CLI fallback.

pub mod core_memory_store;
pub mod memory_store;
pub mod migrate;
pub mod models;
pub mod pool;

pub use core_memory_store::{CoreEntryBuilder, DEFAULT_CORE_MEMORY_BUDGET, SqliteCoreMemoryStore};
pub use memory_store::{EmbeddingProvider, SqliteMemoryStore};
pub use migrate::run_migrations;
pub use pool::init_pool;
pub use sqlx::SqlitePool;
