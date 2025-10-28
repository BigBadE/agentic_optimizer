//! Agent execution with self-assessment and step tracking.
//!
//! This module provides the agent execution infrastructure for running LLM-powered
//! agents with self-assessment capabilities and detailed step tracking.

/// Conversation management for multi-turn interactions
pub mod conversation;
/// Execution result types for TypeScript-based agent system
pub mod execution_result;
/// Agent executor for running LLM-powered agents
pub mod executor;
/// Self-assessment functionality for agents to evaluate their own work
pub mod self_assess;
/// Step tracking for monitoring agent execution progress
pub mod step;
/// Task coordination for complex multi-step workflows
pub mod task_coordinator;

// Re-export context management from merlin-context
pub use conversation::{ConversationManager, ConversationMessage, ConversationSummary};
pub use execution_result::AgentExecutionResult;
pub use executor::AgentExecutor;
pub use merlin_context::ContextFetcher;
pub use merlin_context::context_inclusion::ContextManager;
pub use self_assess::SelfAssessor;
pub use step::StepTracker;
pub use task_coordinator::{CoordinatorStats, TaskCoordinator, TaskProgress, TaskStatus};
