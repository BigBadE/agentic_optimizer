//! Agent execution, task coordination, and validation for LLM-powered agents.
//!
//! This crate provides agent execution capabilities including:
//!
//! - **Agent Execution**: Core agent runtime with context management and conversation tracking
//! - **Task Execution**: Workspace isolation, conflict detection, and parallel execution
//! - **Validation Pipeline**: Multi-stage validation (syntax, lint, test, build)
//!
//! # Architecture
//!
//! The crate is organized into three key modules:
//!
//! - [`agent`]: Agent execution, context management, conversation tracking, and self-assessment
//! - [`executor`]: Task execution with workspace isolation and conflict detection
//! - [`validator`]: Multi-stage validation pipeline for code generation
//!
//! # Example
//!
//! ```no_run
//! use merlin_agent::{AgentExecutor, ValidationPipeline};
//! use merlin_routing::StrategyRouter;
//! use merlin_tooling::ToolRegistry;
//! use merlin_context::ContextFetcher;
//! use merlin_core::RoutingConfig;
//! use std::sync::Arc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let router = Arc::new(StrategyRouter::with_default_strategies()?);
//! let validator = Arc::new(ValidationPipeline::with_default_stages());
//! let tool_registry = Arc::new(ToolRegistry::default());
//! let context_fetcher = ContextFetcher::new(".".into());
//! let config = RoutingConfig::default();
//!
//! let _executor = AgentExecutor::new(router, validator, tool_registry, context_fetcher, config)?;
//! # Ok(())
//! # }
//! ```

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

/// Agent execution and self-assessment
pub mod agent;
/// Task execution with workspace management
pub mod executor;
/// High-level orchestration of routing components
pub mod orchestrator;
/// Validation pipeline and stages
pub mod validator;

pub use agent::{
    AgentExecutor, AgentExecutorWrapper, CommandResult, CommandRunner, ContextFetcher,
    ContextManager, ConversationManager, ConversationMessage, ConversationSummary,
    CoordinatorStats, RealAgentExecutorWrapper, SelfAssessor, StepTracker, TaskCoordinator,
    TaskListExecutor, TaskListResult, TaskProgress as AgentTaskProgress, TaskStatus,
};
pub use executor::{
    BuildResult, ConflictAwareTaskGraph, ConflictReport, ExecutorPool, FileConflict,
    FileLockManager, IsolatedBuildEnv, LintResult, TaskGraph, TaskWorkspace, TestResult,
    WorkspaceSnapshot, WorkspaceState,
};
pub use orchestrator::RoutingOrchestrator;
pub use validator::{
    BuildValidationStage, LintValidationStage, SyntaxValidationStage, TestValidationStage,
    ValidationPipeline, ValidationStage as ValidationStageTrait, Validator,
};
