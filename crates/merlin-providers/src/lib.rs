//! Provider adapters for external LLM services.


pub mod anthropic;
pub mod groq;
pub mod openrouter;

pub use anthropic::AnthropicProvider;
pub use groq::GroqProvider;
pub use openrouter::OpenRouterProvider;
