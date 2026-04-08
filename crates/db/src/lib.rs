//! Database layer for OpenRustClaw.
//!
//! Uses sqlx (async, primary) for general persistence,
//! libSQL for vector operations, and rusqlite for sync CLI fallback.

pub mod core_memory_store;
pub mod learning_store;
pub mod memory_store;
pub mod migrate;
pub mod models;
pub mod pool;
pub mod rag_store;
pub mod session_store;
pub mod skill_proposal_store;

pub use core_memory_store::{CoreEntryBuilder, DEFAULT_CORE_MEMORY_BUDGET, SqliteCoreMemoryStore};
pub use learning_store::SqliteLearningStore;
pub use memory_store::{EmbeddingProvider, SqliteMemoryStore};
pub use migrate::run_migrations;
pub use pool::init_pool;
pub use rag_store::{RagChunkInput, RagChunkRecord, RagCollectionStats, SqliteRagStore};
pub use session_store::{PersistedSession, SessionStatus, SqliteSessionStore};
pub use skill_proposal_store::SqliteSkillProposalStore;
pub use sqlx::SqlitePool;
