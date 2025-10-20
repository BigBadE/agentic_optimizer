//! Task execution with workspace management and conflict detection.
//!
//! This module provides infrastructure for executing tasks in isolation,
//! managing workspace state, tracking file conflicts, and running build/test/lint operations.

/// Build environment isolation and validation
pub mod build_isolation;
/// Task dependency graph construction
pub mod graph;
/// File locking and isolation primitives
pub mod isolation;
/// Executor pool for parallel task execution
pub mod pool;
/// Conflict-aware task scheduler
pub mod scheduler;
/// Workspace state tracking and snapshots
pub mod state;
/// Transactional workspace operations
pub mod transaction;

pub use build_isolation::{BuildResult, IsolatedBuildEnv, LintResult, TestResult};
pub use graph::TaskGraph;
pub use isolation::{FileLockManager, ReadLockGuard, WriteLockGuard};
pub use pool::ExecutorPool;
pub use scheduler::ConflictAwareTaskGraph;
pub use state::{WorkspaceSnapshot, WorkspaceState};
pub use transaction::{CommitResult, ConflictReport, FileConflict, TaskWorkspace};
