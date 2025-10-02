//! Query analysis and intent extraction for context building.

mod types;
mod analyzer;

pub use types::{QueryIntent, Action, Scope, ContextPlan, ExpansionStrategy};
pub use analyzer::QueryAnalyzer;
