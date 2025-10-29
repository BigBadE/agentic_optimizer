//! Core types for tasks, analysis, validation, and execution.
//!
//! This module defines the fundamental types used throughout the routing system,
//! including task representation, complexity/priority levels, validation results,
//! and execution context.

mod analysis;
mod core;
mod decomposition;
mod execution;
mod validation;

// Re-export all public types
pub use analysis::*;
pub use core::*;
pub use decomposition::*;
pub use execution::*;
pub use validation::*;
