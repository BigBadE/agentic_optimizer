//! Core types and traits for the agentic optimizer.
//!
//! This crate provides fundamental types, error handling, and trait definitions
//! used across the agentic optimizer system.

/// Error types and result definitions.
pub mod error;
/// Trait definitions for model providers.
pub mod traits;
/// Core data types for queries, responses, and context.
pub mod types;

pub use error::{Error, Result};
pub use traits::ModelProvider;
pub use types::{Context, FileContext, Query, Response, TokenUsage};
