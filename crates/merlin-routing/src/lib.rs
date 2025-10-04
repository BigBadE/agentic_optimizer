pub mod analyzer;
pub mod config;
pub mod error;
pub mod executor;
pub mod orchestrator;
pub mod router;
pub mod types;
pub mod ui;
pub mod validator;

pub use analyzer::{
    Action, ComplexityEstimator, Intent, IntentExtractor, LocalTaskAnalyzer, Scope,
    TaskAnalyzer, TaskDecomposer,
};
pub use config::{ExecutionConfig, RoutingConfig, TierConfig, ValidationConfig, WorkspaceConfig};
pub use error::{Result, RoutingError};
pub use orchestrator::RoutingOrchestrator;
pub use executor::{
    BuildResult, ConflictAwareTaskGraph, ConflictReport, ExecutorPool, FileConflict,
    FileLockManager, IsolatedBuildEnv, LintResult, TaskGraph, TaskWorkspace, TestResult,
    WorkspaceSnapshot, WorkspaceState,
};
pub use router::{
    AvailabilityChecker, ComplexityBasedStrategy, CostOptimizationStrategy, LongContextStrategy,
    ModelRouter, ModelTier, QualityCriticalStrategy, RoutingDecision, RoutingStrategy,
    StrategyRouter,
};
pub use types::{
    Complexity, ContextRequirements, ExecutionStrategy, FileChange, Priority, Severity,
    StageResult, Task, TaskAnalysis, TaskId, TaskResult, ValidationError, ValidationResult,
    ValidationStage as ValidationStageType,
};
pub use ui::{MessageLevel, TaskProgress, TuiApp, UiChannel, UiEvent};
pub use validator::{
    BuildValidationStage, LintValidationStage, SyntaxValidationStage, TestValidationStage,
    ValidationPipeline, ValidationStage as ValidationStageTrait, Validator,
};
