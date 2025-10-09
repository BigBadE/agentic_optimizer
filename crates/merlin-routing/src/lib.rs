//! Intelligent task routing and orchestration for LLM-powered agents.
//!
//! This crate provides a comprehensive framework for routing tasks to appropriate
//! language models based on complexity, cost, and quality requirements. It includes:
//!
//! - **Task Analysis**: Automatic analysis of task complexity and requirements
//! - **Smart Routing**: Multiple strategies for selecting optimal models (complexity-based,
//!   cost-optimization, quality-critical, long-context)
//! - **Execution Management**: Parallel task execution with dependency tracking and conflict resolution
//! - **Validation Pipeline**: Multi-stage validation (syntax, lint, test, build)
//! - **Interactive UI**: Terminal user interface for monitoring and managing tasks
//! - **Streaming Support**: Real-time streaming of LLM responses and execution progress
//!
//! # Architecture
//!
//! The crate is organized into several key modules:
//!
//! - [`agent`]: Agent execution and self-assessment capabilities
//! - [`analyzer`]: Task complexity analysis and decomposition
//! - [`router`]: Model selection strategies and tier management
//! - [`executor`]: Task execution with workspace isolation and conflict detection
//! - [`validator`]: Multi-stage validation pipeline for code generation
//! - [`orchestrator`]: High-level coordination of all components
//!
//! # Example
//!
//! ```no_run
//! use merlin_routing::{RoutingOrchestrator, RoutingConfig, Task};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = RoutingConfig::default();
//! let orchestrator = RoutingOrchestrator::new(config);
//!
//! let task = Task::new("Implement a new feature".to_owned());
//! let result = orchestrator.execute_task(task).await?;
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
/// Task analysis and decomposition
pub mod analyzer;
/// Response caching with semantic similarity
pub mod cache;
/// Configuration types for routing and execution
pub mod config;
/// Error types and result aliases
pub mod error;
/// Task execution with workspace management
pub mod executor;
/// Metrics collection and reporting
pub mod metrics;
/// High-level orchestration of routing components
pub mod orchestrator;
/// Model selection and routing strategies
pub mod router;
/// Streaming events and channels
pub mod streaming;
/// Tool implementations for file operations and commands
pub mod tools;
/// Core types for tasks, analysis, and validation
pub mod types;
/// Terminal user interface for task management
pub mod user_interface;
/// Validation pipeline and stages
pub mod validator;

pub use agent::{AgentExecutor, SelfAssessor, StepTracker};
pub use analyzer::{
    Action, ComplexityEstimator, Intent, IntentExtractor, LocalTaskAnalyzer, Scope, TaskAnalyzer,
    TaskDecomposer,
};
pub use cache::{CachedResponse, ResponseCache};
pub use config::{
    CacheConfig, ExecutionConfig, RoutingConfig, TierConfig, ValidationConfig, WorkspaceConfig,
};
pub use error::{Result, RoutingError};
pub use executor::{
    BuildResult, ConflictAwareTaskGraph, ConflictReport, ExecutorPool, FileConflict,
    FileLockManager, IsolatedBuildEnv, LintResult, TaskGraph, TaskWorkspace, TestResult,
    WorkspaceSnapshot, WorkspaceState,
};
pub use metrics::{DailyReport, MetricsCollector, MetricsReport, RequestMetrics, TierBreakdown};
pub use orchestrator::RoutingOrchestrator;
pub use router::{
    AvailabilityChecker, ComplexityBasedStrategy, CostOptimizationStrategy, LongContextStrategy,
    ModelRouter, ModelTier, QualityCriticalStrategy, RoutingDecision, RoutingStrategy,
    StrategyRouter,
};
pub use streaming::{StepId, StepType, StreamingChannel, StreamingEvent, TaskStep};
pub use tools::{ListFilesTool, ReadFileTool, RunCommandTool, Tool, ToolRegistry, WriteFileTool};
pub use types::{
    CommandExecution,
    Complexity,
    ContextRequirements,
    ExecutionContext,
    ExecutionMode,
    ExecutionStrategy,
    FileChange,
    Priority,
    Severity,
    StageResult,
    SubtaskSpec,
    Task,
    TaskAction,
    TaskAnalysis,
    TaskDecision,
    TaskId,
    TaskResult,
    // Self-determination types
    TaskState,
    ValidationError,
    ValidationResult,
    ValidationStage as ValidationStageType,
};
pub use user_interface::{MessageLevel, TaskProgress, TuiApp, UiChannel, UiEvent};
pub use validator::{
    BuildValidationStage, LintValidationStage, SyntaxValidationStage, TestValidationStage,
    ValidationPipeline, ValidationStage as ValidationStageTrait, Validator,
};
