//! LLM provider integrations for OpenRustClaw.
//!
//! Each provider uses its native API format via `reqwest` HTTP calls.
//! The fallback chain automatically tries the next provider on rate limits or errors.

pub mod anthropic;
pub mod fallback;
pub mod gemini;
pub mod ollama;
pub mod openai;
pub mod openrouter;
pub mod tool_formats;

pub use anthropic::AnthropicProvider;
pub use fallback::ProviderChain;
pub use gemini::{create_gemini_provider, GeminiProvider};
pub use ollama::OllamaProvider;
pub use openai::OpenAiProvider;
pub use openrouter::OpenRouterProvider;
pub use tool_formats::translate_tool_definition;
