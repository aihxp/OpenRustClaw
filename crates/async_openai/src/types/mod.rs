//! Type definitions for the OpenAI API.

mod chat;
mod embeddings;
mod shared;

pub use chat::{ChatChoice, ChatMessage, ChatResponse, Function, FunctionCall, Role, Tool, ToolCall};
pub use embeddings::{Embedding, EmbeddingUsage, EmbeddingsResponse};
pub use shared::{TokenUsage, FinishReason};
