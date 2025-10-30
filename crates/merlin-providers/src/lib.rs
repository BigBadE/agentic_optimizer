//! Provider adapters for external LLM services.
#![cfg_attr(
    test,
    allow(
        clippy::missing_panics_doc,
        clippy::missing_errors_doc,
        reason = "Allow for tests"
    )
)]

/// Groq provider implementation.
pub mod groq;
/// Mock provider for testing.
pub mod mock;
/// `OpenRouter` multi-provider implementation.
pub mod openrouter;

pub use groq::GroqProvider;
pub use mock::MockProvider;
pub use openrouter::OpenRouterProvider;
