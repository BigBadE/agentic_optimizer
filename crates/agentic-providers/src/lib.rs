//! Provider adapters for external LLM services.

pub mod anthropic;
pub mod openrouter;

pub use anthropic::AnthropicProvider;
pub use openrouter::OpenRouterProvider;
