//! Agent execution with step tracking.
//!
//! This module provides the agent execution infrastructure for running LLM-powered
//! agents with detailed step tracking.

/// Execution result types for TypeScript-based agent system
pub mod execution_result;
/// Agent executor for running LLM-powered agents
pub mod executor;
/// Step tracking for monitoring agent execution progress
pub mod step;

// Re-export context management from merlin-context
pub use execution_result::AgentExecutionResult;
pub use executor::{AgentExecutor, StepExecutionParams, StepExecutor, StepResult};
pub use merlin_context::ContextFetcher;
pub use merlin_context::context_inclusion::ContextManager;
pub use step::StepTracker;

// DEAD CODE REMOVED:
// - conversation.rs (ConversationManager) - Never used in production code, only in unit tests
// - task_coordinator/ (TaskCoordinator) - Never used in production code, only in unit tests
