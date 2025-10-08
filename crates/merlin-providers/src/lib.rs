//! Provider adapters for external LLM services.
#![cfg_attr(
    test,
    allow(
        dead_code,
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::panic,
        clippy::missing_panics_doc,
        clippy::missing_errors_doc,
        clippy::print_stdout,
        clippy::print_stderr,
        reason = "Allow for tests"
    )
)]

/// Anthropic Claude provider implementation.
pub mod anthropic;
/// Groq provider implementation.
pub mod groq;
/// `OpenRouter` multi-provider implementation.
pub mod openrouter;

pub use anthropic::AnthropicProvider;
pub use groq::GroqProvider;
pub use openrouter::OpenRouterProvider;
