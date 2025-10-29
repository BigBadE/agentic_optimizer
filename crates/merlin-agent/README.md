# merlin-agent

Agent execution, task coordination, validation pipeline, and workspace management.

## Purpose

This crate provides the core agent execution engine with task coordination, multi-stage validation, workspace isolation, and transaction support.

Implements recursive step-based execution where agents return either a String result or a TaskList for decomposition. Each step has exit requirements for validation and can recursively decompose further.

## Module Structure

### Agent Execution (`agent/`)
- `executor/` - Agent execution system
  - `mod.rs` - `AgentExecutor` for executing agent tasks with TypeScript code execution
  - `step_executor.rs` - `StepExecutor` for recursive step-based execution
  - `typescript.rs` - TypeScript code extraction and execution
  - `context.rs` - Context building for tasks
- `step.rs` - `StepTracker` for tracking execution steps
- `task_coordinator.rs` - `TaskCoordinator` for coordinating multiple tasks
- `task_list_executor.rs` - `TaskListExecutor` for workflow execution
- `command_runner.rs` - Command execution utilities
- `conversation.rs` - `ConversationManager` for managing conversations
- `execution_result.rs` - Execution result types (string or TaskList)

### Executor System (`executor/`)
- `state.rs` - `WorkspaceState` for state management
- `transaction.rs` - `TaskWorkspace` for transactional operations
- `graph.rs` - `TaskGraph`, `ConflictAwareTaskGraph` for dependency management
- `scheduler.rs` - Task scheduling logic
- `pool.rs` - `ExecutorPool` for parallel execution
- `isolation.rs` - `FileLockManager` for file-level locking
- `build_isolation.rs` - `IsolatedBuildEnv` for isolated builds

### Validation Pipeline (`validator/`)
- `pipeline.rs` - `ValidationPipeline` orchestrator
- `citations.rs` - Citation validation
- Stages:
  - `syntax.rs` - Syntax validation
  - `lint.rs` - Linting validation
  - `test.rs` - Test execution
  - `build.rs` - Build validation

### Exit Requirement Validators (`exit_validators.rs`)
- Built-in callback validators for step completion
- `file_exists`, `file_contains`, `command_succeeds`, `json_valid`, `no_errors_in`
- Pattern matching and named validator integration

## Public API

**Agent System:**
- `AgentExecutor` - Execute agent tasks with TypeScript runtime
- `StepExecutor` - Recursive step-based execution with exit requirements
- `ExitRequirementValidators` - Built-in validators for step completion
- `StepTracker` - Track execution steps
- `TaskCoordinator` - Coordinate multiple tasks
- `TaskListExecutor` - Execute multi-step workflows
- `ConversationManager`, `ContextManager` - Conversation management
- `AgentExecutionResult` - String result or continuation request

**Executor System:**
- `TaskGraph`, `ConflictAwareTaskGraph` - Dependency graphs
- `ExecutorPool` - Parallel task execution
- `FileLockManager` - File locking
- `IsolatedBuildEnv` - Isolated build environments
- `WorkspaceState`, `WorkspaceSnapshot`, `TaskWorkspace` - State management

**Validation:**
- `ValidationPipeline` - Multi-stage validation
- Validation stages: `SyntaxStage`, `LintStage`, `TestStage`, `BuildStage`

## Features

### Task Coordination

- Agent returns `String | TaskList`
- Recursive decomposition - steps can return TaskLists
- Exit requirements validate each step completion
- Retry logic with hard/soft error classification
- Context specification per step (files, previous results, explicit content)
- Full tool access at all times
- Dependency tracking
- Conflict detection
- Parallel execution support

### Workspace Isolation
- Transactional file operations
- Snapshot-based rollback
- File-level locking (RAII guards)
- Conflict-aware scheduling

### Validation Pipeline
- Multi-stage validation (Syntax → Lint → Test → Build)
- Early exit on failure
- Isolated build environments
- Comprehensive reporting

### Conversation Management
- Multi-turn conversations
- Context management
- History tracking

## Testing Status

**✅ Well-tested**

- **Unit tests**: 6 files with comprehensive coverage
  - `executor.rs`, `step.rs`, `self_assess.rs`, `task_list_executor.rs`
  - `transaction.rs`, `state.rs`
- **Fixture coverage**: 20+ fixtures
  - `agent/` - Agent execution tests, including TypeScript-based self-determination
    - `self_determination_complete.json` - Complete action testing
    - `self_determination_decompose.json` - Decompose action with subtasks
    - `self_determination_gather.json` - GatherContext action
  - `executor/` - Task execution tests
  - `validation/` - Validation pipeline tests
  - `workspace/` - Workspace isolation tests

## Code Quality

- ✅ **Documentation**: All public items documented
- ✅ **Error handling**: Proper `Result<T, E>` usage
- ✅ **No dead code**: All modules actively used
- ✅ **No TODOs**: Implementation complete

## Dependencies

- `merlin-core` - Core types
- `merlin-tooling` - Tool system
- `merlin-routing` - Task routing
- `serde` - Serialization
- `tokio` - Async runtime
- `tempfile` - Temporary directories

## Usage Example

```rust
use merlin_agent::{AgentExecutor, TaskCoordinator, ValidationPipeline};
use merlin_core::Task;

// Execute single task
let executor = AgentExecutor::new();
let result = executor.execute(&task).await?;

// Coordinate multiple tasks
let coordinator = TaskCoordinator::new();
let results = coordinator.execute_all(&tasks).await?;

// Validate changes
let pipeline = ValidationPipeline::new();
let validation_result = pipeline.validate(&workspace).await?;
```

## Issues and Recommendations

**None** - This crate is well-maintained with excellent test coverage and comprehensive fixture-based testing.
