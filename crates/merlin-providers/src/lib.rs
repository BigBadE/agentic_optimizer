//! Provider adapters for external LLM services.

/// Anthropic Claude provider implementation.
pub mod anthropic;
/// Groq provider implementation.
pub mod groq;
/// `OpenRouter` multi-provider implementation.
pub mod openrouter;

pub use anthropic::AnthropicProvider;
pub use groq::GroqProvider;
pub use openrouter::OpenRouterProvider;
