//! Provider adapters for external LLM services.

/// Groq provider implementation.
pub mod groq;
/// `OpenRouter` multi-provider implementation.
pub mod openrouter;

pub use groq::GroqProvider;
pub use openrouter::OpenRouterProvider;
