//! Memory, RAG, and context management for OpenRustClaw.
//!
//! Three-tier database-first architecture:
//! - **Core Memory**: Tiny, always in prompt (~500 tokens)
//! - **Recall Memory**: Searchable via memory_search tool, never auto-injected
//! - **Archive**: Consolidated long-term summaries

pub mod archive;
pub mod artifacts;
pub mod context;
pub mod core_memory;
pub mod embeddings;
pub mod model_artifacts;
pub mod policies;
pub mod rag;
pub mod recall;
pub mod search;

pub use artifacts::{ResolvedArtifactBundle, WorkspaceArtifact, WorkspaceArtifactRegistry};
pub use context::ContextManager;
pub use core_memory::CoreMemoryManager;
pub use model_artifacts::{
    MODEL_ARTIFACT_MAX_SUMMARY_CHARS, ModelArtifactService, build_model_artifact_projections,
    reserved_model_artifact_core_keys,
};
pub use policies::{AssistantMemoryWriteBasis, AssistantMemoryWriteDecision, MemoryPolicies};
pub use recall::RecallMemory;
