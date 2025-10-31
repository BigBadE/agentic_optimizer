//! Provider adapters for external LLM services.

/// Groq provider implementation.
pub mod groq;
/// Mock provider for testing.
pub mod mock;
/// `OpenRouter` multi-provider implementation.
pub mod openrouter;

pub use groq::GroqProvider;
pub use mock::MockProvider;
pub use openrouter::OpenRouterProvider;
