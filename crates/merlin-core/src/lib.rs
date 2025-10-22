//! Core types and traits for the agentic optimizer.
//!
//! This crate provides fundamental types, error handling, and trait definitions
//! used across the agentic optimizer system.
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
        reason = "Test allows"
    )
)]

/// Error types and result definitions.
pub mod error;
/// Prompt loading utilities.
pub mod prompts;
/// Trait definitions for model providers.
pub mod traits;
/// Core data types for queries, responses, and context.
pub mod types;

// Modules from merlin-types (now merged into merlin-core)
/// Configuration types
pub mod config;
/// Routing error types
pub mod routing_error;
/// Streaming events and step tracking
pub mod streaming;
/// Task types and execution context
pub mod task;
/// Task list structure for multi-step workflows
pub mod task_list;
/// UI event system
pub mod ui;

// Original merlin-core exports
pub use error::Error;
pub use error::Result as CoreResult; // Renamed to avoid conflict
pub use traits::ModelProvider;
pub use types::{Context, FileContext, Query, Response, TokenUsage};

// Re-export types from merged modules (formerly merlin-types)
pub use config::{
    CacheConfig, ExecutionConfig, RoutingConfig, TaskListCommands, TierConfig, ValidationConfig,
    WorkspaceConfig,
};
pub use routing_error::RoutingError;
// Re-export Result from routing_error as the main Result (for backward compatibility with merlin-types)
pub use routing_error::Result;
pub use streaming::{StepId, StepType, StreamingChannel, StreamingEvent, TaskStep};
pub use task::{
    CommandExecution, ContextRequirements, ExecutionContext, ExecutionMode, ExecutionStrategy,
    FileChange, Priority, Severity, StageResult, SubtaskSpec, Task, TaskAction, TaskAnalysis,
    TaskDecision, TaskId, TaskResult, TaskState, ValidationError, ValidationResult,
    ValidationStage as ValidationStageType,
};
pub use task_list::{
    StepStatus, StepType as TaskStepType, TaskList, TaskListStatus, TaskStep as TaskListStep,
};
pub use ui::{MessageLevel, TaskProgress, UiChannel, UiEvent};
