//! Agent execution with self-assessment and step tracking.
//!
//! This module provides the agent execution infrastructure for running LLM-powered
//! agents with self-assessment capabilities and detailed step tracking.

/// Context fetching with file reference extraction
pub mod context_fetcher;
/// Agent executor for running LLM-powered agents
pub mod executor;
/// Self-assessment functionality for agents to evaluate their own work
pub mod self_assess;
/// Step tracking for monitoring agent execution progress
pub mod step;

pub use context_fetcher::ContextFetcher;
pub use executor::AgentExecutor;
pub use self_assess::SelfAssessor;
pub use step::StepTracker;
