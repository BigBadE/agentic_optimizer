//! Core types and traits for the agentic optimizer.
//!
//! This crate provides fundamental types, error handling, and trait definitions
//! used across the agentic optimizer system.

/// Error types and result definitions.
pub mod error;
/// Prompt loading utilities.
pub mod prompts;
/// Synchronization utilities.
pub mod sync;
/// Trait definitions for model providers.
pub mod traits;
/// Core data types for queries, responses, and context.
pub mod types;

// Modules from merlin-types (now merged into merlin-core)
/// Configuration types
pub mod config;
/// Conversation threading system
pub mod conversation;
/// Routing error types
pub mod routing_error;
/// Streaming events and step tracking
pub mod streaming;
/// Task types and execution context
pub mod task;
/// UI event system
pub mod ui;

// Original merlin-core exports
pub use error::Error;
pub use error::Result as CoreResult; // Renamed to avoid conflict
pub use sync::IgnoreLock;
pub use traits::ModelProvider;
pub use types::{Context, FileContext, Query, Response, TokenUsage};

// Re-export types from merged modules (formerly merlin-types)
pub use config::{
    ProjectConfig, RoutingConfig, TierConfig, ValidationCheckType, ValidationChecks,
    ValidationConfig,
};
pub use conversation::{
    BranchPoint, Message, MessageId, Subtask, SubtaskId, SubtaskStatus, Thread, ThreadColor,
    ThreadId, VerificationStep, WorkStatus, WorkUnit, WorkUnitId,
};
pub use routing_error::RoutingError;
// Re-export Result from routing_error as the main Result (for backward compatibility with merlin-types)
pub use routing_error::Result;
pub use streaming::{ExecutionStep, ExecutionStepType, StepId, StreamingChannel, StreamingEvent};
pub use task::{
    // Task list execution model types
    AgentResponse,
    // Original types
    CommandExecution,
    ContextRequirements,
    ContextSpec,
    ExecutionContext,
    ExecutionMode,
    ExecutionStrategy,
    FileChange,
    FilePattern,
    JsValueHandle,
    Priority,
    Severity,
    StageResult,
    StepType,
    Task,
    TaskAction,
    TaskAnalysis,
    TaskDecision,
    TaskId,
    TaskList,
    TaskResult,
    TaskState,
    TaskStep,
    ValidationError,
    ValidationErrorType,
    ValidationResult,
    ValidationStage as ValidationStageType,
};
pub use ui::{MessageLevel, TaskProgress, UiChannel, UiEvent};
