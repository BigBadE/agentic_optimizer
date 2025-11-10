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
//! use merlin_routing::StrategyRouter;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a router with default configuration
//! let _router = StrategyRouter::with_default_strategies()?;
//! # Ok(())
//! # }
//! ```

/// Task analysis and decomposition
pub mod analyzer;
/// Response caching with semantic similarity
pub mod cache;
/// Metrics collection and reporting
pub mod metrics;
/// Model selection and routing strategies
pub mod router;
/// Terminal user interface for task management
pub mod user_interface;

// Re-export types from merlin-core for backward compatibility
pub use merlin_core::{
    CommandExecution, ContextRequirements, ExecutionContext, ExecutionMode, ExecutionStrategy,
    FileChange, MessageLevel, Priority, Result, RoutingConfig, RoutingError, Severity, StageResult,
    StepId, StepType, StreamingChannel, StreamingEvent, Subtask, Task, TaskAction, TaskAnalysis,
    TaskDecision, TaskId, TaskProgress, TaskResult, TaskState, TaskStep, TierConfig, UiChannel,
    UiEvent, ValidationConfig, ValidationError, ValidationResult, ValidationStageType,
};

pub use analyzer::{
    Action, Intent, IntentExtractor, LocalTaskAnalyzer, Scope, TaskAnalyzer, TaskDecomposer,
};
pub use cache::{CacheStats, CachedResponse, ResponseCache};
pub use metrics::{
    DailyReport, MetricsCollector, MetricsReport, RequestMetrics, RequestMetricsParams,
    TierBreakdown,
};
pub use router::{
    AvailabilityChecker, Model, ModelRegistry, ModelRouter, ProviderRegistry, RoutingDecision,
    StrategyRouter, TierCategory,
};
// Re-export tools from merlin-tools and merlin-typescript crates
pub use merlin_tooling::{
    BashTool, Tool, ToolRegistry, TypeScriptRuntime, generate_typescript_signatures,
};
