//! Provider adapters for external LLM services.

/// Claude Code provider implementation.
pub mod claude_code;
/// Groq provider implementation.
pub mod groq;
/// `OpenRouter` multi-provider implementation.
pub mod openrouter;

pub use claude_code::ClaudeCodeProvider;
pub use groq::GroqProvider;
pub use openrouter::OpenRouterProvider;
