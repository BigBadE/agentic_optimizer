//! Agent execution, task coordination, and validation for LLM-powered agents.
//!
//! This crate provides agent execution capabilities including:
//!
//! - **Agent Execution**: Core agent runtime with context management and conversation tracking
//! - **Parallel Execution**: Dependency-aware parallel task execution
//! - **Validation Pipeline**: Multi-stage validation (syntax, lint, test, build)
//!
//! # Architecture
//!
//! The crate is organized into three key modules:
//!
//! - [`agent`]: Agent execution, context management, conversation tracking, and parallel execution
//! - [`validator`]: Multi-stage validation pipeline for code generation
//! - [`orchestrator`]: High-level routing orchestration
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
//! let tool_registry = ToolRegistry::default();
//! let context_fetcher = ContextFetcher::new(".".into());
//! let config = RoutingConfig::default();
//!
//! let _executor = AgentExecutor::new(router, validator, tool_registry, context_fetcher, &config)?;
//! # Ok(())
//! # }
//! ```

/// Agent execution and self-assessment
pub mod agent;
/// High-level orchestration of routing components
pub mod orchestrator;
/// Thread persistence and management
pub mod thread_store;
/// Validation pipeline and stages
pub mod validator;

pub use agent::{
    AgentExecutor, ContextFetcher, ContextManager, StepExecutionParams, StepExecutor, StepResult,
    StepTracker,
};
pub use orchestrator::RoutingOrchestrator;
pub use thread_store::ThreadStore;
pub use validator::{
    SyntaxValidationStage, ValidationPipeline, ValidationStage as ValidationStageTrait, Validator,
};
