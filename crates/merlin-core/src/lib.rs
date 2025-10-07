//! Core types and traits for the agentic optimizer.
//!
//! This crate provides fundamental types, error handling, and trait definitions
//! used across the agentic optimizer system.


pub mod error;
pub mod traits;
pub mod types;

pub use error::{Error, Result};
pub use traits::ModelProvider;
pub use types::{Context, FileContext, Query, Response, TokenUsage};
