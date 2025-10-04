# Multi-Model Routing Architecture

## Implementation Checklist

### Phase 1: Core Types & Traits ✅
- [x] **merlin-routing/src/types.rs** - Core types (TaskId, Task, Complexity, Priority, etc.)
- [x] **merlin-routing/src/error.rs** - RoutingError enum
- [x] **merlin-routing/Cargo.toml** - Crate dependencies
- [x] **merlin-routing/src/lib.rs** - Crate root with re-exports
- [x] **merlin-routing/src/analyzer/mod.rs** - TaskAnalyzer trait
- [x] **merlin-routing/src/router/mod.rs** - ModelRouter trait, ModelTier, RoutingDecision
- [x] **merlin-routing/src/validator/mod.rs** - Validator trait, ValidationResult

### Phase 2: Task Graph & Execution ✅
- [x] **merlin-routing/src/executor/graph.rs** - TaskGraph with dependency tracking
- [x] **merlin-routing/src/executor/state.rs** - WorkspaceState with RwLock
- [x] **merlin-routing/src/executor/pool.rs** - ExecutorPool with parallel execution
- [x] **merlin-routing/src/executor/mod.rs** - Re-exports

### Phase 3: Isolation & Conflict Management ✅
- [x] **merlin-routing/src/executor/isolation.rs** - FileLockManager with RAII guards
- [x] **merlin-routing/src/executor/transaction.rs** - TaskWorkspace with snapshots
- [x] **merlin-routing/src/executor/build_isolation.rs** - IsolatedBuildEnv with TempDir
- [x] **merlin-routing/src/executor/scheduler.rs** - ConflictAwareTaskGraph

### Phase 4: UI Layer ✅
- [x] **merlin-routing/src/ui/mod.rs** - TuiApp, UiChannel, UiEvent
- [x] **merlin-routing/src/ui/events.rs** - UI event types
- [x] **Type-system enforcement** - UiChannel required in ExecutorPool

### Phase 5: Routing Strategies ✅
- [x] **merlin-routing/src/router/strategy.rs** - RoutingStrategy trait
- [x] **merlin-routing/src/router/strategies/complexity.rs** - ComplexityBasedStrategy
- [x] **merlin-routing/src/router/strategies/cost.rs** - CostOptimizationStrategy
- [x] **merlin-routing/src/router/strategies/quality.rs** - QualityCriticalStrategy
- [x] **merlin-routing/src/router/strategies/context.rs** - LongContextStrategy
- [x] **merlin-routing/src/router/tiers.rs** - StrategyRouter with availability checking
- [x] **merlin-routing/src/router/strategies/mod.rs** - Strategy re-exports

### Phase 6: Validation Pipeline ✅
- [x] **merlin-routing/src/validator/pipeline.rs** - ValidationPipeline with early exit
- [x] **merlin-routing/src/validator/stages/syntax.rs** - SyntaxValidationStage (heuristics)
- [x] **merlin-routing/src/validator/stages/build.rs** - BuildValidationStage (cargo check)
- [x] **merlin-routing/src/validator/stages/test.rs** - TestValidationStage (cargo test)
- [x] **merlin-routing/src/validator/stages/lint.rs** - LintValidationStage (clippy)
- [x] **merlin-routing/src/validator/stages/mod.rs** - Stage re-exports

### Phase 7: Task Analysis ✅
- [x] **merlin-routing/src/analyzer/intent.rs** - Intent extraction (keyword-based)
- [x] **merlin-routing/src/analyzer/complexity.rs** - Complexity estimation (multi-factor)
- [x] **merlin-routing/src/analyzer/decompose.rs** - Task decomposition (smart splitting)
- [x] **merlin-routing/src/analyzer/local.rs** - LocalTaskAnalyzer implementation (no LLM)

### Phase 8: Local Model Integration ✅
- [x] **merlin-local/Cargo.toml** - New crate for local models
- [x] **merlin-local/src/lib.rs** - Crate root with re-exports
- [x] **merlin-local/src/manager.rs** - Ollama integration and model management
- [x] **merlin-local/src/models.rs** - Model metadata and API types
- [x] **merlin-local/src/inference.rs** - LocalModelProvider implementation
- [x] **merlin-local/src/error.rs** - Local model errors

### Phase 9: Groq Provider ✅
- [x] **merlin-providers/src/groq.rs** - Groq API client (free tier)
- [x] **merlin-providers/src/lib.rs** - Add Groq export

### Phase 10: Orchestrator ✅
- [x] **merlin-routing/src/orchestrator.rs** - High-level coordinator
- [x] **merlin-routing/src/config.rs** - RoutingConfig with all sub-configs
- [x] **merlin-routing/src/lib.rs** - Complete re-exports

### Phase 11: Integration & Testing ✅
- [x] **merlin-routing/tests/integration_tests.rs** - Integration test framework (TODO: implement tests)
- [x] **merlin-routing/examples/basic_routing.rs** - Complete example
- [x] **merlin-routing/README.md** - Comprehensive documentation
- [x] **Update merlin-cli** - Integrate routing system with `route` command
- [x] **docs/CLI_ROUTING.md** - CLI routing documentation
- [x] **README.md** - Updated with routing examples

---

## TODO: Remaining Work

### High Priority (Production Ready) ✅ COMPLETE
- [x] **Provider Integration** - Connect orchestrator to actual providers
  - [x] Implement provider factory in orchestrator
  - [x] Map ModelTier to concrete providers (LocalModelProvider, GroqProvider, etc.)
  - [x] Handle provider initialization and API keys
  - [x] Add provider fallback logic with escalation
- [x] **Real Execution** - Replace mock results with actual model calls
  - [x] Remove mock response generation
  - [x] Execute tasks through selected providers
  - [x] Handle provider errors and retries (up to 3 attempts)
  - [x] Implement escalation on failure (automatic tier upgrade)
- [x] **TUI Mode** - Interactive routing interface
  - [x] Add `--tui` flag to route command
  - [x] Integrate TuiApp with orchestrator
  - [x] Real-time progress updates
  - [x] Task status display

### Medium Priority (Enhanced Features)
- [ ] **Config Files** - TOML/JSON configuration support
  - [ ] Load config from `~/.agentic/config.toml`
  - [ ] Project-specific `.agentic.toml`
  - [ ] Environment variable overrides
- [ ] **Response Caching** - Cache responses for identical queries
  - [ ] Implement cache key generation
  - [ ] Store responses in local cache
  - [ ] Cache invalidation strategy
- [ ] **Metrics Tracking** - Track costs, latency, success rates
  - [ ] Record all executions to database
  - [ ] Generate daily/weekly reports
  - [ ] Cost analysis and optimization suggestions

### Low Priority (Future Enhancements)
- [ ] **Streaming Responses** - Support streaming for real-time feedback
- [ ] **Multi-turn Conversations** - Maintain conversation context
- [ ] **Custom Strategies** - Plugin system for routing strategies
- [ ] **Learning System** - Adjust routing based on historical performance
- [ ] **Integration Tests** - Comprehensive end-to-end tests using valor
  - See `tests/integration_tests.rs` for test scenarios

---

## Design Principles

### Core Tenets
1. **Decoupling**: Components communicate through traits, not concrete types
2. **Minimal Contracts**: Interfaces expose only essential operations
3. **Concurrency Safety**: Immutable data where possible, explicit synchronization where needed
4. **Type Safety**: Leverage Rust's type system to prevent misuse
5. **Testability**: Each component independently testable
6. **Extensibility**: New models/strategies pluggable without core changes

### Anti-Patterns to Avoid
- ❌ Shared mutable state without synchronization
- ❌ Tight coupling between routing logic and execution
- ❌ Leaking implementation details across module boundaries
- ❌ Global singletons for task coordination
- ❌ Blocking operations in async contexts
- ❌ **Concurrent file modifications without conflict detection**
- ❌ **Global build state shared across parallel tasks**
- ❌ **Committing broken intermediate states**

---

## Critical: State Isolation & Conflict Management

### The Problem

Parallel tasks can corrupt each other's work:

**Scenario 1: File Conflict**
```
Task A: Refactor function foo() in utils.rs
Task B: Add feature using foo() in utils.rs
→ Both modify utils.rs simultaneously → Corruption
```

**Scenario 2: Build State**
```
Task A: Refactor breaking the build temporarily
Task B: Tries to run tests while build is broken
→ Task B fails spuriously due to Task A's incomplete work
```

**Scenario 3: Cascading Changes**
```
Task A: Rename type Foo → Bar
Task B: Add usage of type Foo
→ Task B's changes invalid after Task A commits
```

### Solution: Multi-Layer Isolation

#### Layer 1: File-Level Locking

**File**: `merlin-routing/src/executor/isolation.rs`

```rust
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use tokio::sync::RwLock;

/// Tracks which files are being modified by which tasks
pub struct FileLockManager {
    /// Maps file paths to the task holding exclusive write lock
    write_locks: RwLock<HashMap<PathBuf, TaskId>>,
    /// Maps file paths to set of tasks holding read locks
    read_locks: RwLock<HashMap<PathBuf, HashSet<TaskId>>>,
}

impl FileLockManager {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            write_locks: RwLock::new(HashMap::new()),
            read_locks: RwLock::new(HashMap::new()),
        })
    }
    
    /// Acquire write lock on files (exclusive access)
    pub async fn acquire_write_locks(
        &self,
        task_id: TaskId,
        files: &[PathBuf],
    ) -> Result<WriteLockGuard> {
        let mut write_locks = self.write_locks.write().await;
        let read_locks = self.read_locks.read().await;
        
        // Check if any file is already locked
        for file in files {
            if let Some(holder) = write_locks.get(file) {
                if *holder != task_id {
                    return Err(Error::FileLockedByTask {
                        file: file.clone(),
                        holder: *holder,
                    });
                }
            }
            
            if let Some(readers) = read_locks.get(file) {
                if !readers.is_empty() && !readers.contains(&task_id) {
                    return Err(Error::FileHasActiveReaders {
                        file: file.clone(),
                        readers: readers.len(),
                    });
                }
            }
        }
        
        // Acquire locks
        for file in files {
            write_locks.insert(file.clone(), task_id);
        }
        
        Ok(WriteLockGuard {
            manager: self,
            task_id,
            files: files.to_vec(),
        })
    }
    
    /// Acquire read lock on files (shared access)
    pub async fn acquire_read_locks(
        &self,
        task_id: TaskId,
        files: &[PathBuf],
    ) -> Result<ReadLockGuard> {
        let write_locks = self.write_locks.read().await;
        let mut read_locks = self.read_locks.write().await;
        
        // Check if any file has exclusive write lock
        for file in files {
            if let Some(holder) = write_locks.get(file) {
                if *holder != task_id {
                    return Err(Error::FileLockedByTask {
                        file: file.clone(),
                        holder: *holder,
                    });
                }
            }
        }
        
        // Acquire read locks
        for file in files {
            read_locks
                .entry(file.clone())
                .or_insert_with(HashSet::new)
                .insert(task_id);
        }
        
        Ok(ReadLockGuard {
            manager: self,
            task_id,
            files: files.to_vec(),
        })
    }
}

/// RAII guard for write locks - released on drop
pub struct WriteLockGuard<'a> {
    manager: &'a FileLockManager,
    task_id: TaskId,
    files: Vec<PathBuf>,
}

impl Drop for WriteLockGuard<'_> {
    fn drop(&mut self) {
        // Release locks (async in tokio::spawn)
        let manager = self.manager.clone();
        let files = self.files.clone();
        tokio::spawn(async move {
            let mut write_locks = manager.write_locks.write().await;
            for file in files {
                write_locks.remove(&file);
            }
        });
    }
}
```

#### Layer 2: Transactional Workspace

**File**: `merlin-routing/src/executor/transaction.rs`

```rust
use std::collections::HashMap;
use std::path::PathBuf;

/// Isolated workspace for a single task
pub struct TaskWorkspace {
    /// Base state when task started
    base_snapshot: Arc<WorkspaceSnapshot>,
    /// Pending changes (not yet committed)
    pending_changes: HashMap<PathBuf, FileState>,
    /// Files locked by this task
    lock_guard: WriteLockGuard,
}

#[derive(Debug, Clone)]
pub enum FileState {
    Created(String),
    Modified(String),
    Deleted,
}

impl TaskWorkspace {
    /// Create isolated workspace for task
    pub async fn new(
        task_id: TaskId,
        files_to_modify: Vec<PathBuf>,
        global_state: Arc<WorkspaceState>,
        lock_manager: Arc<FileLockManager>,
    ) -> Result<Self> {
        // Acquire exclusive locks on files
        let lock_guard = lock_manager
            .acquire_write_locks(task_id, &files_to_modify)
            .await?;
        
        // Snapshot current state
        let base_snapshot = global_state.snapshot(&files_to_modify).await?;
        
        Ok(Self {
            base_snapshot,
            pending_changes: HashMap::new(),
            lock_guard,
        })
    }
    
    /// Modify file in isolated workspace
    pub fn modify_file(&mut self, path: PathBuf, content: String) {
        self.pending_changes.insert(path, FileState::Modified(content));
    }
    
    /// Read file (sees pending changes + base snapshot)
    pub fn read_file(&self, path: &PathBuf) -> Option<String> {
        // Check pending changes first
        if let Some(state) = self.pending_changes.get(path) {
            return match state {
                FileState::Created(content) | FileState::Modified(content) => {
                    Some(content.clone())
                }
                FileState::Deleted => None,
            };
        }
        
        // Fall back to base snapshot
        self.base_snapshot.get(path)
    }
    
    /// Validate changes don't conflict with current global state
    pub async fn check_conflicts(
        &self,
        global_state: Arc<WorkspaceState>,
    ) -> Result<ConflictReport> {
        let mut conflicts = Vec::new();
        
        for (path, _) in &self.pending_changes {
            let base_version = self.base_snapshot.get(path);
            let current_version = global_state.read_file(path).await;
            
            // Check if file changed since we started
            if base_version != current_version {
                conflicts.push(FileConflict {
                    path: path.clone(),
                    base_hash: hash_content(&base_version),
                    current_hash: hash_content(&current_version),
                });
            }
        }
        
        Ok(ConflictReport { conflicts })
    }
    
    /// Commit changes to global state (atomic)
    pub async fn commit(
        self,
        global_state: Arc<WorkspaceState>,
    ) -> Result<CommitResult> {
        // Check for conflicts before committing
        let conflict_report = self.check_conflicts(global_state.clone()).await?;
        
        if !conflict_report.conflicts.is_empty() {
            return Err(Error::ConflictDetected(conflict_report));
        }
        
        // Apply all changes atomically
        let changes: Vec<FileChange> = self.pending_changes
            .into_iter()
            .map(|(path, state)| match state {
                FileState::Created(content) => FileChange::Create { path, content },
                FileState::Modified(content) => FileChange::Modify { path, content },
                FileState::Deleted => FileChange::Delete { path },
            })
            .collect();
        
        global_state.apply_changes(&changes).await?;
        
        // Locks released automatically on drop
        Ok(CommitResult {
            files_changed: changes.len(),
        })
    }
    
    /// Abort changes (rollback)
    pub async fn rollback(self) -> Result<()> {
        // Simply drop - locks released, changes discarded
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct WorkspaceSnapshot {
    files: HashMap<PathBuf, String>,
}

impl WorkspaceSnapshot {
    pub fn get(&self, path: &PathBuf) -> Option<String> {
        self.files.get(path).cloned()
    }
}

#[derive(Debug)]
pub struct ConflictReport {
    pub conflicts: Vec<FileConflict>,
}

#[derive(Debug)]
pub struct FileConflict {
    pub path: PathBuf,
    pub base_hash: u64,
    pub current_hash: u64,
}
```

#### Layer 3: Build State Isolation

**File**: `merlin-routing/src/executor/build_isolation.rs`

```rust
use std::path::PathBuf;
use tempfile::TempDir;

/// Isolated build environment for task validation
pub struct IsolatedBuildEnv {
    /// Temporary directory with workspace copy
    temp_dir: TempDir,
    /// Original workspace path
    original_workspace: PathBuf,
}

impl IsolatedBuildEnv {
    /// Create isolated build environment
    pub async fn new(workspace: &WorkspaceState) -> Result<Self> {
        let temp_dir = TempDir::new()?;
        
        // Copy workspace to temp directory
        Self::copy_workspace(workspace, temp_dir.path()).await?;
        
        Ok(Self {
            temp_dir,
            original_workspace: workspace.root_path().to_path_buf(),
        })
    }
    
    /// Apply changes to isolated environment
    pub async fn apply_changes(&self, changes: &[FileChange]) -> Result<()> {
        for change in changes {
            match change {
                FileChange::Create { path, content } |
                FileChange::Modify { path, content } => {
                    let full_path = self.temp_dir.path().join(path);
                    tokio::fs::write(full_path, content).await?;
                }
                FileChange::Delete { path } => {
                    let full_path = self.temp_dir.path().join(path);
                    tokio::fs::remove_file(full_path).await.ok();
                }
            }
        }
        Ok(())
    }
    
    /// Run build validation in isolation
    pub async fn validate_build(&self) -> Result<BuildResult> {
        let output = tokio::process::Command::new("cargo")
            .arg("check")
            .arg("--all-targets")
            .current_dir(self.temp_dir.path())
            .timeout(Duration::from_secs(60))
            .output()
            .await?;
        
        Ok(BuildResult {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }
    
    /// Run tests in isolation
    pub async fn run_tests(&self) -> Result<TestResult> {
        let output = tokio::process::Command::new("cargo")
            .arg("test")
            .arg("--all-targets")
            .current_dir(self.temp_dir.path())
            .timeout(Duration::from_secs(300))
            .output()
            .await?;
        
        Ok(TestResult {
            success: output.status.success(),
            passed: Self::parse_test_count(&output.stdout, "passed"),
            failed: Self::parse_test_count(&output.stdout, "failed"),
            details: String::from_utf8_lossy(&output.stdout).to_string(),
        })
    }
}

// Cleanup happens automatically on Drop (TempDir)
```

#### Layer 4: Dependency-Aware Scheduling

**File**: `merlin-routing/src/executor/scheduler.rs`

```rust
/// Enhanced task graph with file conflict detection
pub struct ConflictAwareTaskGraph {
    graph: TaskGraph,
    file_access_map: HashMap<PathBuf, Vec<TaskId>>,
}

impl ConflictAwareTaskGraph {
    pub fn from_tasks(tasks: Vec<Task>) -> Self {
        let mut file_access_map: HashMap<PathBuf, Vec<TaskId>> = HashMap::new();
        
        // Build file access map
        for task in &tasks {
            for file in &task.context_needs.required_files {
                file_access_map
                    .entry(file.clone())
                    .or_insert_with(Vec::new)
                    .push(task.id);
            }
        }
        
        let graph = TaskGraph::from_tasks(tasks);
        
        Self {
            graph,
            file_access_map,
        }
    }
    
    /// Get ready tasks that don't conflict with running tasks
    pub fn ready_non_conflicting_tasks(
        &self,
        completed: &HashSet<TaskId>,
        running: &HashSet<TaskId>,
    ) -> Vec<Task> {
        let base_ready = self.graph.ready_tasks(completed);
        
        // Filter out tasks that would conflict with running tasks
        base_ready
            .into_iter()
            .filter(|task| {
                !self.conflicts_with_running(task, running)
            })
            .collect()
    }
    
    fn conflicts_with_running(&self, task: &Task, running: &HashSet<TaskId>) -> bool {
        // Check if any of task's files are accessed by running tasks
        for file in &task.context_needs.required_files {
            if let Some(accessing_tasks) = self.file_access_map.get(file) {
                for other_task_id in accessing_tasks {
                    if running.contains(other_task_id) && *other_task_id != task.id {
                        return true; // Conflict detected
                    }
                }
            }
        }
        false
    }
}
```

### Updated ExecutorPool with Isolation

```rust
pub struct ExecutorPool {
    router: Arc<dyn ModelRouter>,
    validator: Arc<dyn Validator>,
    max_concurrent: usize,
    workspace: Arc<WorkspaceState>,
    lock_manager: Arc<FileLockManager>,  // NEW
    isolation_mode: IsolationMode,        // NEW
}

#[derive(Debug, Clone, Copy)]
pub enum IsolationMode {
    /// No isolation - fastest but unsafe
    None,
    /// File-level locking only
    FileLocking,
    /// Transactional workspaces per task
    Transactional,
    /// Full isolation with temp build environments
    FullIsolation,
}

impl ExecutorPool {
    pub async fn execute_graph(&self, graph: TaskGraph) -> Result<Vec<TaskResult>> {
        if graph.has_cycles() {
            return Err(Error::CyclicDependency);
        }
        
        // Convert to conflict-aware graph
        let conflict_graph = ConflictAwareTaskGraph::from_tasks(graph.tasks);
        
        let mut completed = HashSet::new();
        let mut running = HashSet::new();  // NEW: Track running tasks
        let mut results = Vec::new();
        let mut join_set = JoinSet::new();
        
        loop {
            // Get tasks that are ready AND don't conflict with running tasks
            let ready = conflict_graph.ready_non_conflicting_tasks(
                &completed,
                &running,
            );
            
            if ready.is_empty() && join_set.is_empty() {
                break;
            }
            
            // Spawn non-conflicting tasks
            for task in ready {
                if join_set.len() >= self.max_concurrent {
                    break;
                }
                
                running.insert(task.id);
                
                let router = self.router.clone();
                let validator = self.validator.clone();
                let workspace = self.workspace.clone();
                let lock_manager = self.lock_manager.clone();
                let isolation_mode = self.isolation_mode;
                
                join_set.spawn(async move {
                    let result = Self::execute_task_isolated(
                        task,
                        router,
                        validator,
                        workspace,
                        lock_manager,
                        isolation_mode,
                    ).await;
                    result
                });
            }
            
            // Wait for a task to complete
            if let Some(result) = join_set.join_next().await {
                let task_result = result??;
                completed.insert(task_result.task_id);
                running.remove(&task_result.task_id);  // No longer running
                results.push(task_result);
            }
        }
        
        Ok(results)
    }
    
    async fn execute_task_isolated(
        task: Task,
        router: Arc<dyn ModelRouter>,
        validator: Arc<dyn Validator>,
        global_workspace: Arc<WorkspaceState>,
        lock_manager: Arc<FileLockManager>,
        isolation_mode: IsolationMode,
    ) -> Result<TaskResult> {
        match isolation_mode {
            IsolationMode::None => {
                // Direct execution (unsafe but fast)
                Self::execute_task_direct(task, router, validator, global_workspace).await
            }
            IsolationMode::FileLocking => {
                // Acquire locks, execute, release
                Self::execute_task_locked(
                    task,
                    router,
                    validator,
                    global_workspace,
                    lock_manager,
                ).await
            }
            IsolationMode::Transactional => {
                // Use transactional workspace
                Self::execute_task_transactional(
                    task,
                    router,
                    validator,
                    global_workspace,
                    lock_manager,
                ).await
            }
            IsolationMode::FullIsolation => {
                // Isolated build environment
                Self::execute_task_fully_isolated(
                    task,
                    router,
                    validator,
                    global_workspace,
                    lock_manager,
                ).await
            }
        }
    }
    
    async fn execute_task_transactional(
        task: Task,
        router: Arc<dyn ModelRouter>,
        validator: Arc<dyn Validator>,
        global_workspace: Arc<WorkspaceState>,
        lock_manager: Arc<FileLockManager>,
    ) -> Result<TaskResult> {
        const MAX_RETRIES: usize = 3;
        let mut attempt = 0;
        
        loop {
            attempt += 1;
            
            // Create isolated workspace
            let mut task_workspace = TaskWorkspace::new(
                task.id,
                task.context_needs.required_files.clone(),
                global_workspace.clone(),
                lock_manager.clone(),
            ).await?;
            
            // Execute task
            let response = Self::execute_on_tier_with_workspace(
                &task,
                &router,
                &mut task_workspace,
            ).await?;
            
            // Validate in isolation
            let build_env = IsolatedBuildEnv::new(&global_workspace).await?;
            let changes = Self::extract_changes(&response)?;
            build_env.apply_changes(&changes).await?;
            
            let validation = if task.requires_build_check() {
                let build_result = build_env.validate_build().await?;
                validator.validate_with_build(&response, &task, &build_result).await?
            } else {
                validator.validate(&response, &task).await?
            };
            
            if validation.passed {
                // Try to commit (may fail if conflicts)
                match task_workspace.commit(global_workspace.clone()).await {
                    Ok(_) => {
                        return Ok(TaskResult {
                            task_id: task.id,
                            response,
                            tier_used: ModelTier::Local, // placeholder
                            validation,
                        });
                    }
                    Err(Error::ConflictDetected(report)) => {
                        // Another task modified files - retry
                        if attempt >= MAX_RETRIES {
                            return Err(Error::MaxConflictRetries { task_id: task.id, report });
                        }
                        // Retry with updated base state
                        continue;
                    }
                    Err(e) => return Err(e),
                }
            } else {
                // Validation failed - rollback and escalate
                task_workspace.rollback().await?;
                // Escalate logic here...
                return Err(Error::ValidationFailed(validation));
            }
        }
    }
}
```

---

## User Interface Layer (Ratatui TUI)

### Design Philosophy

**Critical Requirement**: All tasks MUST provide user feedback. This is enforced at compile-time via the type system.

**UI Principles:**
1. **Non-blocking**: UI updates never block task execution
2. **Real-time**: Users see what's happening as it happens
3. **Scrollable**: All output preserved and scrollable
4. **Hierarchical**: Tasks show parent-child relationships
5. **Type-safe**: Impossible to create task without feedback channel

### Core TUI Architecture

**File**: `merlin-routing/src/ui/mod.rs`

```rust
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Scrollbar},
    Terminal,
};
use crossterm::event::{self, Event, KeyCode};
use tokio::sync::mpsc;

/// UI event that tasks send to update display
#[derive(Debug, Clone)]
pub enum UiEvent {
    TaskStarted {
        task_id: TaskId,
        description: String,
        parent_id: Option<TaskId>,
    },
    TaskProgress {
        task_id: TaskId,
        progress: TaskProgress,
    },
    TaskOutput {
        task_id: TaskId,
        output: String,
    },
    TaskCompleted {
        task_id: TaskId,
        result: TaskResult,
    },
    TaskFailed {
        task_id: TaskId,
        error: String,
    },
    SystemMessage {
        level: MessageLevel,
        message: String,
    },
}

#[derive(Debug, Clone)]
pub struct TaskProgress {
    pub stage: String,
    pub current: u64,
    pub total: Option<u64>,
    pub message: String,
}

#[derive(Debug, Clone, Copy)]
pub enum MessageLevel {
    Info,
    Warning,
    Error,
    Success,
}

/// UI update channel - REQUIRED for all task execution
pub struct UiChannel {
    sender: mpsc::UnboundedSender<UiEvent>,
}

impl UiChannel {
    pub fn send(&self, event: UiEvent) {
        // Best-effort send, don't block on UI
        let _ = self.sender.send(event);
    }
    
    /// Convenience method for task started
    pub fn task_started(&self, task_id: TaskId, description: String) {
        self.send(UiEvent::TaskStarted {
            task_id,
            description,
            parent_id: None,
        });
    }
    
    /// Convenience method for progress updates
    pub fn progress(&self, task_id: TaskId, stage: String, message: String) {
        self.send(UiEvent::TaskProgress {
            task_id,
            progress: TaskProgress {
                stage,
                current: 0,
                total: None,
                message,
            },
        });
    }
    
    /// Convenience method for output
    pub fn output(&self, task_id: TaskId, output: String) {
        self.send(UiEvent::TaskOutput { task_id, output });
    }
    
    /// Convenience method for completion
    pub fn completed(&self, task_id: TaskId, result: TaskResult) {
        self.send(UiEvent::TaskCompleted { task_id, result });
    }
    
    /// Convenience method for errors
    pub fn failed(&self, task_id: TaskId, error: String) {
        self.send(UiEvent::TaskFailed { task_id, error });
    }
}

/// Main TUI application state
pub struct TuiApp {
    terminal: Terminal<CrosstermBackend<std::io::Stdout>>,
    event_receiver: mpsc::UnboundedReceiver<UiEvent>,
    state: UiState,
}

#[derive(Default)]
struct UiState {
    tasks: HashMap<TaskId, TaskDisplay>,
    task_order: Vec<TaskId>,
    output_buffer: Vec<OutputLine>,
    scroll_offset: usize,
    selected_task: Option<TaskId>,
    input_buffer: String,
    input_mode: InputMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputMode {
    Normal,
    Editing,
}

struct TaskDisplay {
    id: TaskId,
    description: String,
    status: TaskStatus,
    progress: Option<TaskProgress>,
    children: Vec<TaskId>,
    output_lines: Vec<String>,
    start_time: Instant,
    end_time: Option<Instant>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Validating,
    Retrying,
}

struct OutputLine {
    task_id: Option<TaskId>,
    timestamp: Instant,
    level: MessageLevel,
    text: String,
}

impl TuiApp {
    pub fn new() -> (Self, UiChannel) {
        let (sender, receiver) = mpsc::unbounded_channel();
        
        let terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))
            .expect("Failed to create terminal");
        
        let app = Self {
            terminal,
            event_receiver: receiver,
            state: UiState::default(),
        };
        
        let channel = UiChannel { sender };
        
        (app, channel)
    }
    
    /// Run TUI event loop (spawned as separate task)
    pub async fn run(mut self) -> Result<()> {
        loop {
            // Handle UI events from tasks
            while let Ok(event) = self.event_receiver.try_recv() {
                self.handle_event(event);
            }
            
            // Render UI
            self.render()?;
            
            // Handle user input
            if event::poll(Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    match self.state.input_mode {
                        InputMode::Normal => {
                            match key.code {
                                KeyCode::Char('q') => break,
                                KeyCode::Char('i') => {
                                    self.state.input_mode = InputMode::Editing;
                                }
                                KeyCode::Up => self.scroll_up(),
                                KeyCode::Down => self.scroll_down(),
                                KeyCode::PageUp => self.page_up(),
                                KeyCode::PageDown => self.page_down(),
                                _ => {}
                            }
                        }
                        InputMode::Editing => {
                            match key.code {
                                KeyCode::Esc => {
                                    self.state.input_mode = InputMode::Normal;
                                }
                                KeyCode::Enter => {
                                    self.submit_input();
                                    self.state.input_mode = InputMode::Normal;
                                }
                                KeyCode::Backspace => {
                                    self.state.input_buffer.pop();
                                }
                                KeyCode::Char(c) => {
                                    self.state.input_buffer.push(c);
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        
        Ok(())
    }
    
    fn handle_event(&mut self, event: UiEvent) {
        match event {
            UiEvent::TaskStarted { task_id, description, parent_id } => {
                let task_display = TaskDisplay {
                    id: task_id,
                    description: description.clone(),
                    status: TaskStatus::Running,
                    progress: None,
                    children: Vec::new(),
                    output_lines: Vec::new(),
                    start_time: Instant::now(),
                    end_time: None,
                };
                
                self.state.tasks.insert(task_id, task_display);
                self.state.task_order.push(task_id);
                
                if let Some(parent) = parent_id {
                    if let Some(parent_task) = self.state.tasks.get_mut(&parent) {
                        parent_task.children.push(task_id);
                    }
                }
                
                self.state.output_buffer.push(OutputLine {
                    task_id: Some(task_id),
                    timestamp: Instant::now(),
                    level: MessageLevel::Info,
                    text: format!("▶ Started: {}", description),
                });
            }
            
            UiEvent::TaskProgress { task_id, progress } => {
                if let Some(task) = self.state.tasks.get_mut(&task_id) {
                    task.progress = Some(progress.clone());
                    task.output_lines.push(format!("  {} - {}", progress.stage, progress.message));
                }
                
                self.state.output_buffer.push(OutputLine {
                    task_id: Some(task_id),
                    timestamp: Instant::now(),
                    level: MessageLevel::Info,
                    text: format!("  {} - {}", progress.stage, progress.message),
                });
            }
            
            UiEvent::TaskOutput { task_id, output } => {
                if let Some(task) = self.state.tasks.get_mut(&task_id) {
                    task.output_lines.push(output.clone());
                }
                
                self.state.output_buffer.push(OutputLine {
                    task_id: Some(task_id),
                    timestamp: Instant::now(),
                    level: MessageLevel::Info,
                    text: output,
                });
            }
            
            UiEvent::TaskCompleted { task_id, result } => {
                if let Some(task) = self.state.tasks.get_mut(&task_id) {
                    task.status = TaskStatus::Completed;
                    task.end_time = Some(Instant::now());
                }
                
                self.state.output_buffer.push(OutputLine {
                    task_id: Some(task_id),
                    timestamp: Instant::now(),
                    level: MessageLevel::Success,
                    text: format!("✓ Completed: {} ({}ms)", 
                        self.state.tasks.get(&task_id).map_or("", |t| &t.description),
                        result.duration_ms),
                });
            }
            
            UiEvent::TaskFailed { task_id, error } => {
                if let Some(task) = self.state.tasks.get_mut(&task_id) {
                    task.status = TaskStatus::Failed;
                    task.end_time = Some(Instant::now());
                }
                
                self.state.output_buffer.push(OutputLine {
                    task_id: Some(task_id),
                    timestamp: Instant::now(),
                    level: MessageLevel::Error,
                    text: format!("✗ Failed: {}", error),
                });
            }
            
            UiEvent::SystemMessage { level, message } => {
                self.state.output_buffer.push(OutputLine {
                    task_id: None,
                    timestamp: Instant::now(),
                    level,
                    text: message,
                });
            }
        }
    }
    
    fn render(&mut self) -> Result<()> {
        self.terminal.draw(|frame| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),        // Header
                    Constraint::Percentage(30),   // Scrollable output (lower priority)
                    Constraint::Min(10),          // Task list (main focus)
                    Constraint::Length(3),        // User input box
                    Constraint::Length(3),        // Status bar
                ])
                .split(frame.size());
            
            // Render header
            self.render_header(frame, chunks[0]);
            
            // Render scrollable output (moved to top, lower priority)
            self.render_output(frame, chunks[1]);
            
            // Render task list with progress bars (main focus)
            self.render_task_list(frame, chunks[2]);
            
            // Render user input box
            self.render_input_box(frame, chunks[3]);
            
            // Render status bar
            self.render_status_bar(frame, chunks[4]);
        })?;
        
        Ok(())
    }
    
    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let running = self.state.tasks.values()
            .filter(|t| t.status == TaskStatus::Running)
            .count();
        let completed = self.state.tasks.values()
            .filter(|t| t.status == TaskStatus::Completed)
            .count();
        let failed = self.state.tasks.values()
            .filter(|t| t.status == TaskStatus::Failed)
            .count();
        
        let header = Paragraph::new(format!(
            "Merlin - Tasks: {} running | {} completed | {} failed",
            running, completed, failed
        ))
        .block(Block::default().borders(Borders::ALL).title("Status"));
        
        frame.render_widget(header, area);
    }
    
    fn render_task_list(&self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self.state.task_order
            .iter()
            .filter_map(|task_id| self.state.tasks.get(task_id))
            .map(|task| self.render_task_item(task))
            .collect();
        
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Active Tasks"));
        
        frame.render_widget(list, area);
    }
    
    fn render_task_item(&self, task: &TaskDisplay) -> ListItem {
        let status_icon = match task.status {
            TaskStatus::Pending => "○",
            TaskStatus::Running => "▶",
            TaskStatus::Completed => "✓",
            TaskStatus::Failed => "✗",
            TaskStatus::Validating => "⧗",
            TaskStatus::Retrying => "↻",
        };
        
        let elapsed = task.end_time
            .unwrap_or_else(Instant::now)
            .duration_since(task.start_time)
            .as_millis();
        
        let mut text = format!(
            "{} {} ({}ms)",
            status_icon,
            task.description,
            elapsed
        );
        
        // Add progress bar if available
        if let Some(progress) = &task.progress {
            text.push_str(&format!("\n    └─ {}: {}", progress.stage, progress.message));
            
            if let Some(total) = progress.total {
                let percent = (progress.current as f64 / total as f64 * 100.0) as u16;
                text.push_str(&format!(" [{}%]", percent));
            }
        }
        
        ListItem::new(text)
    }
    
    fn render_output(&self, frame: &mut Frame, area: Rect) {
        let visible_lines = area.height as usize - 2; // Account for borders
        let start = self.state.scroll_offset;
        let end = (start + visible_lines).min(self.state.output_buffer.len());
        
        let lines: Vec<String> = self.state.output_buffer[start..end]
            .iter()
            .map(|line| {
                let timestamp = format!("[{:02}:{:02}]",
                    line.timestamp.elapsed().as_secs() / 60,
                    line.timestamp.elapsed().as_secs() % 60);
                
                let level_prefix = match line.level {
                    MessageLevel::Info => "ℹ",
                    MessageLevel::Warning => "⚠",
                    MessageLevel::Error => "✗",
                    MessageLevel::Success => "✓",
                };
                
                format!("{} {} {}", timestamp, level_prefix, line.text)
            })
            .collect();
        
        let paragraph = Paragraph::new(lines.join("\n"))
            .block(Block::default()
                .borders(Borders::ALL)
                .title(format!("Output (scroll: {}/{})", start, self.state.output_buffer.len())))
            .scroll((0, 0));
        
        frame.render_widget(paragraph, area);
        
        // Render scrollbar
        if self.state.output_buffer.len() > visible_lines {
            let scrollbar = Scrollbar::default();
            frame.render_stateful_widget(
                scrollbar,
                area,
                &mut ScrollbarState::new(self.state.output_buffer.len())
                    .position(self.state.scroll_offset),
            );
        }
    }
    
    fn render_input_box(&self, frame: &mut Frame, area: Rect) {
        let input_text = if self.state.input_mode == InputMode::Editing {
            format!("> {}_", self.state.input_buffer)
        } else {
            format!("> {} (press 'i' to edit)", self.state.input_buffer)
        };
        
        let input = Paragraph::new(input_text)
            .block(Block::default()
                .borders(Borders::ALL)
                .title(match self.state.input_mode {
                    InputMode::Normal => "Input (Normal Mode)",
                    InputMode::Editing => "Input (Editing - ESC to cancel, ENTER to submit)",
                }));
        
        frame.render_widget(input, area);
    }
    
    fn render_status_bar(&self, frame: &mut Frame, area: Rect) {
        let help = match self.state.input_mode {
            InputMode::Normal => {
                "i: Input | ↑/↓: Scroll | PgUp/PgDn: Page | q: Quit"
            }
            InputMode::Editing => {
                "ESC: Cancel | ENTER: Submit"
            }
        };
        
        let status = Paragraph::new(help)
            .block(Block::default().borders(Borders::ALL));
        
        frame.render_widget(status, area);
    }
    
    fn submit_input(&mut self) {
        if !self.state.input_buffer.is_empty() {
            // Send user input as system message
            self.state.output_buffer.push(OutputLine {
                task_id: None,
                timestamp: Instant::now(),
                level: MessageLevel::Info,
                text: format!("User input: {}", self.state.input_buffer),
            });
            
            // TODO: Send input to orchestrator for processing
            // This would trigger new task analysis and execution
            
            self.state.input_buffer.clear();
        }
    }
    
    fn scroll_up(&mut self) {
        self.state.scroll_offset = self.state.scroll_offset.saturating_sub(1);
    }
    
    fn scroll_down(&mut self) {
        let max_scroll = self.state.output_buffer.len().saturating_sub(1);
        self.state.scroll_offset = (self.state.scroll_offset + 1).min(max_scroll);
    }
    
    fn page_up(&mut self) {
        self.state.scroll_offset = self.state.scroll_offset.saturating_sub(10);
    }
    
    fn page_down(&mut self) {
        let max_scroll = self.state.output_buffer.len().saturating_sub(1);
        self.state.scroll_offset = (self.state.scroll_offset + 10).min(max_scroll);
    }
}
```

### Type-System Enforcement: UiChannel is REQUIRED

**File**: `merlin-routing/src/executor/pool.rs` (Updated)

```rust
/// ExecutorPool now REQUIRES UiChannel - cannot be constructed without it
pub struct ExecutorPool {
    router: Arc<dyn ModelRouter>,
    validator: Arc<dyn Validator>,
    max_concurrent: usize,
    workspace: Arc<WorkspaceState>,
    lock_manager: Arc<FileLockManager>,
    isolation_mode: IsolationMode,
    ui_channel: UiChannel,  // REQUIRED - no Option<>
}

impl ExecutorPool {
    /// Constructor REQUIRES UiChannel - compile error if not provided
    pub fn new(
        router: Arc<dyn ModelRouter>,
        validator: Arc<dyn Validator>,
        max_concurrent: usize,
        ui_channel: UiChannel,  // NOT optional
    ) -> Self {
        Self {
            router,
            validator,
            max_concurrent,
            workspace: WorkspaceState::new(),
            lock_manager: FileLockManager::new(),
            isolation_mode: IsolationMode::Transactional,
            ui_channel,
        }
    }
    
    pub async fn execute_graph(&self, graph: TaskGraph) -> Result<Vec<TaskResult>> {
        // Send system message at start
        self.ui_channel.send(UiEvent::SystemMessage {
            level: MessageLevel::Info,
            message: format!("Starting execution of {} tasks", graph.task_count()),
        });
        
        // ... existing code ...
        
        for task in ready {
            // Send task started event BEFORE spawning
            self.ui_channel.task_started(task.id, task.description.clone());
            
            let ui_channel = self.ui_channel.clone();
            
            join_set.spawn(async move {
                let result = Self::execute_task_with_feedback(
                    task,
                    router,
                    validator,
                    workspace,
                    lock_manager,
                    isolation_mode,
                    ui_channel,  // Pass to every task
                ).await;
                result
            });
        }
        
        // ... existing code ...
    }
    
    /// Every task execution gets UiChannel - NOT optional
    async fn execute_task_with_feedback(
        task: Task,
        router: Arc<dyn ModelRouter>,
        validator: Arc<dyn Validator>,
        global_workspace: Arc<WorkspaceState>,
        lock_manager: Arc<FileLockManager>,
        isolation_mode: IsolationMode,
        ui: UiChannel,  // REQUIRED parameter
    ) -> Result<TaskResult> {
        // Progress: Analyzing
        ui.progress(task.id, "Analyzing".to_string(), "Determining model tier".to_string());
        
        let routing_decision = router.route(&task).await?;
        
        // Progress: Routing
        ui.progress(
            task.id,
            "Routing".to_string(),
            format!("Selected tier: {:?}", routing_decision.tier),
        );
        
        // Progress: Acquiring locks
        ui.progress(task.id, "Locking".to_string(), "Acquiring file locks".to_string());
        
        let mut task_workspace = TaskWorkspace::new(
            task.id,
            task.context_needs.required_files.clone(),
            global_workspace.clone(),
            lock_manager.clone(),
        ).await?;
        
        // Progress: Executing
        ui.progress(
            task.id,
            "Executing".to_string(),
            format!("Running on {}", routing_decision.tier),
        );
        
        let response = Self::execute_on_tier_with_workspace(
            &task,
            &router,
            &mut task_workspace,
            &ui,  // Pass UI down
        ).await?;
        
        // Output the response
        ui.output(task.id, format!("Generated {} tokens", response.tokens_used.total()));
        
        // Progress: Validating
        ui.progress(task.id, "Validating".to_string(), "Running validation pipeline".to_string());
        
        let build_env = IsolatedBuildEnv::new(&global_workspace).await?;
        let changes = Self::extract_changes(&response)?;
        build_env.apply_changes(&changes).await?;
        
        let validation = validator.validate_with_feedback(&response, &task, &ui).await?;
        
        if validation.passed {
            // Progress: Committing
            ui.progress(task.id, "Committing".to_string(), "Applying changes".to_string());
            
            match task_workspace.commit(global_workspace.clone()).await {
                Ok(_) => {
                    ui.completed(task.id, TaskResult {
                        task_id: task.id,
                        response: response.clone(),
                        tier_used: routing_decision.tier,
                        validation: validation.clone(),
                        duration_ms: start.elapsed().as_millis() as u64,
                    });
                    
                    Ok(TaskResult {
                        task_id: task.id,
                        response,
                        tier_used: routing_decision.tier,
                        validation,
                        duration_ms: start.elapsed().as_millis() as u64,
                    })
                }
                Err(Error::ConflictDetected(report)) => {
                    ui.progress(
                        task.id,
                        "Retrying".to_string(),
                        format!("Conflict detected on {} files", report.conflicts.len()),
                    );
                    
                    // Retry logic...
                    Err(Error::ConflictDetected(report))
                }
                Err(e) => {
                    ui.failed(task.id, e.to_string());
                    Err(e)
                }
            }
        } else {
            ui.failed(task.id, format!("Validation failed: {:?}", validation.errors));
            Err(Error::ValidationFailed(validation))
        }
    }
}
```

### Validator Trait Extension with Feedback

**File**: `merlin-routing/src/validator/mod.rs` (Updated)

```rust
#[async_trait]
pub trait Validator: Send + Sync {
    /// Validate with UI feedback
    async fn validate_with_feedback(
        &self,
        response: &Response,
        task: &Task,
        ui: &UiChannel,
    ) -> Result<ValidationResult>;
    
    // Old method for backwards compatibility
    async fn validate(&self, response: &Response, task: &Task) -> Result<ValidationResult> {
        // Use dummy UI channel
        let (_, ui) = mpsc::unbounded_channel();
        self.validate_with_feedback(response, task, &UiChannel { sender: ui }).await
    }
}

impl ValidationPipeline {
    async fn validate_with_feedback(
        &self,
        response: &Response,
        task: &Task,
        ui: &UiChannel,
    ) -> Result<ValidationResult> {
        let mut result = ValidationResult::default();
        
        for (idx, stage) in self.stages.iter().enumerate() {
            let stage_name = stage.name();
            
            ui.progress(
                task.id,
                format!("Validation {}/{}", idx + 1, self.stages.len()),
                format!("Running {}", stage_name),
            );
            
            let stage_result = stage.validate(response, task).await?;
            
            ui.output(
                task.id,
                format!("  {} - {}: {}", 
                    if stage_result.passed { "✓" } else { "✗" },
                    stage_name,
                    stage_result.details
                ),
            );
            
            result.stages.push(stage_result.clone());
            result.score *= stage_result.score;
            result.passed &= stage_result.passed;
            
            if !stage_result.passed && self.early_exit {
                break;
            }
        }
        
        Ok(result)
    }
}
```

### Main Application Integration

**File**: `merlin-routing/src/orchestrator.rs`

```rust
use tokio::task::JoinHandle;

pub struct Orchestrator {
    analyzer: Arc<dyn TaskAnalyzer>,
    router: Arc<dyn ModelRouter>,
    validator: Arc<dyn Validator>,
    config: RoutingConfig,
}

impl Orchestrator {
    /// Execute user request with TUI
    pub async fn execute_with_ui(&self, request: &str) -> Result<Vec<TaskResult>> {
        // Create TUI and get UI channel
        let (tui_app, ui_channel) = TuiApp::new();
        
        // Spawn TUI in background
        let tui_handle: JoinHandle<Result<()>> = tokio::spawn(async move {
            tui_app.run().await
        });
        
        // Send initial message
        ui_channel.send(UiEvent::SystemMessage {
            level: MessageLevel::Info,
            message: format!("Analyzing request: {}", request),
        });
        
        // Analyze request
        let analysis = self.analyzer.analyze(request).await?;
        
        ui_channel.send(UiEvent::SystemMessage {
            level: MessageLevel::Info,
            message: format!("Decomposed into {} tasks", analysis.tasks.len()),
        });
        
        // Create executor with UI channel
        let executor = ExecutorPool::new(
            self.router.clone(),
            self.validator.clone(),
            self.config.max_concurrent_tasks,
            ui_channel.clone(),  // REQUIRED
        );
        
        // Execute tasks
        let graph = TaskGraph::from_tasks(analysis.tasks);
        let results = executor.execute_graph(graph).await?;
        
        // Send completion message
        ui_channel.send(UiEvent::SystemMessage {
            level: MessageLevel::Success,
            message: format!("Completed {} tasks successfully", results.len()),
        });
        
        // Wait a bit for user to see results, then shutdown
        tokio::time::sleep(Duration::from_secs(2)).await;
        
        // TUI will exit when user presses 'q'
        tui_handle.abort();
        
        Ok(results)
    }
}
```

### Type System Guarantees

**Compile-Time Enforcement:**

```rust
// ✅ This compiles - UI channel provided
let executor = ExecutorPool::new(
    router,
    validator,
    4,
    ui_channel,
);

// ❌ This FAILS to compile - missing UI channel
let executor = ExecutorPool::new(
    router,
    validator,
    4,
    // Missing ui_channel parameter - COMPILE ERROR
);

// ❌ This FAILS to compile - cannot create task execution without UI
async fn execute_task(task: Task) {
    // No way to call execute_task_with_feedback without UiChannel
    // The function signature requires it
}
```

**Key Design Decisions:**

1. **UiChannel is NOT `Option<UiChannel>`** - It's always required
2. **All task execution methods take `UiChannel`** - No way to bypass
3. **Validators get UI channel** - Must report progress
4. **Clone is cheap** - `UiChannel` is just a channel sender
5. **Non-blocking sends** - UI never blocks task execution
6. **Structured data** - Events are strongly typed enums

This makes it **impossible** to execute tasks without providing user feedback. The type system enforces it at compile time.

### TUI Layout (5 Sections)

```
┌─────────────────────────────────────────┐
│ Status: 2 running | 5 completed | 0 failed│  ← Header
├─────────────────────────────────────────┤
│ Output (scroll: 45/128):                 │  ← Scrollable output
│ [00:12] ℹ ▶ Started: Refactor utils.rs  │     (lower priority,
│ [00:13] ℹ   Analyzing - Determining tier │      at top, 30%)
│ [00:14] ✓ ✓ Completed (1234ms)          │
├─────────────────────────────────────────┤
│ Active Tasks:                            │  ← Task list with
│ ▶ Refactor utils.rs (1234ms)            │     progress bars
│   └─ Validating: Running clippy [80%]   │     (main focus,
│ ✓ Add tests (567ms)                     │      flexible size)
├─────────────────────────────────────────┤
│ Input (Normal Mode):                     │  ← User input box
│ > (press 'i' to edit)                    │     (above status)
├─────────────────────────────────────────┤
│ i: Input | ↑/↓: Scroll | q: Quit        │  ← Status bar
└─────────────────────────────────────────┘

When editing (press 'i'):
┌─────────────────────────────────────────┐
│ Input (Editing - ESC to cancel, ENTER to submit):
│ > Fix the build errors_                 │  ← Cursor shown
├─────────────────────────────────────────┤
│ ESC: Cancel | ENTER: Submit             │  ← Context help
└─────────────────────────────────────────┘
```

**Input Modes:**
- **Normal Mode**: Navigate with arrow keys, press 'i' to enter input
- **Editing Mode**: Type freely, ESC to cancel, ENTER to submit

**Key Bindings:**
- **Normal Mode**: `i` (input), `↑/↓` (scroll), `PgUp/PgDn` (page), `q` (quit)
- **Editing Mode**: `ESC` (cancel), `ENTER` (submit), `Backspace` (delete)

---

## Crate Structure

```
merlin/
├── merlin-core/              # Core traits, no routing logic
├── merlin-providers/         # Cloud provider implementations
├── merlin-local/             # NEW: Local model management
├── merlin-routing/           # NEW: Multi-model routing
│   ├── analyzer/              # Task analysis
│   ├── router/                # Model selection
│   ├── executor/              # Task execution
│   ├── validator/             # Validation pipeline
│   └── orchestrator.rs        # High-level coordinator
└── merlin-context/           # Context management (existing)
```

---

## Core Abstractions

### 1. Task Analysis Layer

**File**: `merlin-routing/src/analyzer/mod.rs`

```rust
use async_trait::async_trait;
use std::sync::Arc;
use std::path::PathBuf;

/// Immutable task representation
#[derive(Debug, Clone)]
pub struct Task {
    pub id: TaskId,
    pub description: String,
    pub complexity: Complexity,
    pub priority: Priority,
    pub dependencies: Vec<TaskId>,
    pub context_needs: ContextRequirements,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(uuid::Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Complexity {
    Trivial,
    Simple,
    Medium,
    Complex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone)]
pub struct ContextRequirements {
    pub estimated_tokens: usize,
    pub required_files: Vec<PathBuf>,
    pub requires_full_context: bool,
}

#[derive(Debug, Clone)]
pub struct TaskAnalysis {
    pub tasks: Vec<Task>,
    pub execution_strategy: ExecutionStrategy,
}

#[derive(Debug, Clone, Copy)]
pub enum ExecutionStrategy {
    Sequential,
    Parallel { max_concurrent: usize },
    Pipeline,
}

#[async_trait]
pub trait TaskAnalyzer: Send + Sync {
    async fn analyze(&self, request: &str) -> Result<TaskAnalysis>;
    fn estimate_complexity(&self, request: &str) -> Complexity;
}
```

**Design Decisions:**
- **Immutable `Task`**: Once created, never modified
- **Trait-based**: Swap analysis strategies easily
- **No execution logic**: Analyzer only produces data structures

---

### 2. Model Routing Layer

**File**: `merlin-routing/src/router/mod.rs`

```rust
use async_trait::async_trait;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelTier {
    Local { model_name: String },
    Groq { model_name: String },
    Premium { provider: String, model_name: String },
}

impl ModelTier {
    pub fn escalate(&self) -> Option<Self> {
        match self {
            Self::Local { .. } => Some(Self::Groq {
                model_name: "llama-3.1-70b-versatile".to_string(),
            }),
            Self::Groq { .. } => Some(Self::Premium {
                provider: "openrouter".to_string(),
                model_name: "deepseek/deepseek-coder".to_string(),
            }),
            Self::Premium { model_name, .. } if model_name.contains("deepseek") => {
                Some(Self::Premium {
                    provider: "openrouter".to_string(),
                    model_name: "anthropic/claude-3-haiku".to_string(),
                })
            }
            Self::Premium { model_name, .. } if model_name.contains("haiku") => {
                Some(Self::Premium {
                    provider: "anthropic".to_string(),
                    model_name: "claude-3.5-sonnet".to_string(),
                })
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RoutingDecision {
    pub tier: ModelTier,
    pub estimated_cost: f64,
    pub estimated_latency_ms: u64,
    pub reasoning: String,
}

#[async_trait]
pub trait ModelRouter: Send + Sync {
    async fn route(&self, task: &Task) -> Result<RoutingDecision>;
    async fn is_available(&self, tier: &ModelTier) -> bool;
}

#[async_trait]
pub trait RoutingStrategy: Send + Sync {
    fn applies_to(&self, task: &Task) -> bool;
    async fn select_tier(&self, task: &Task) -> Result<ModelTier>;
    fn priority(&self) -> u8;
}

pub struct StrategyRouter {
    strategies: Vec<Arc<dyn RoutingStrategy>>,
    availability_checker: Arc<AvailabilityChecker>,
}
```

**Design Decisions:**
- **Strategy Pattern**: Multiple routing strategies by priority
- **Explicit Escalation**: Clear upgrade path via `escalate()`
- **Availability Check**: Don't route to unavailable tiers
- **Immutable Decisions**: Routing produces decision object

---

### 3. Parallel Execution Layer

**File**: `merlin-routing/src/executor/graph.rs`

```rust
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct TaskGraph {
    graph: DiGraph<Task, ()>,
    node_map: HashMap<TaskId, NodeIndex>,
}

impl TaskGraph {
    pub fn from_tasks(tasks: Vec<Task>) -> Self {
        let mut graph = DiGraph::new();
        let mut node_map = HashMap::new();
        
        for task in tasks {
            let node = graph.add_node(task.clone());
            node_map.insert(task.id, node);
        }
        
        for task in graph.node_weights() {
            let task_node = node_map[&task.id];
            for dep_id in &task.dependencies {
                if let Some(&dep_node) = node_map.get(dep_id) {
                    graph.add_edge(dep_node, task_node, ());
                }
            }
        }
        
        Self { graph, node_map }
    }
    
    pub fn ready_tasks(&self, completed: &HashSet<TaskId>) -> Vec<Task> {
        self.graph
            .node_indices()
            .filter_map(|node| {
                let task = &self.graph[node];
                
                if completed.contains(&task.id) {
                    return None;
                }
                
                let deps_satisfied = self.graph
                    .edges_directed(node, petgraph::Direction::Incoming)
                    .all(|edge| {
                        let dep_task = &self.graph[edge.source()];
                        completed.contains(&dep_task.id)
                    });
                
                if deps_satisfied {
                    Some(task.clone())
                } else {
                    None
                }
            })
            .collect()
    }
    
    pub fn is_complete(&self, completed: &HashSet<TaskId>) -> bool {
        self.graph.node_count() == completed.len()
    }
    
    pub fn has_cycles(&self) -> bool {
        petgraph::algo::is_cyclic_directed(&self.graph)
    }
}
```

**File**: `merlin-routing/src/executor/pool.rs`

```rust
use tokio::sync::{RwLock, Semaphore};
use tokio::task::JoinSet;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct TaskResult {
    pub task_id: TaskId,
    pub response: Response,
    pub tier_used: ModelTier,
    pub validation: ValidationResult,
}

pub struct WorkspaceState {
    files: RwLock<HashMap<PathBuf, String>>,
    metadata: RwLock<HashMap<String, serde_json::Value>>,
}

impl WorkspaceState {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            files: RwLock::new(HashMap::new()),
            metadata: RwLock::new(HashMap::new()),
        })
    }
    
    pub async fn apply_changes(&self, changes: &[FileChange]) -> Result<()> {
        let mut files = self.files.write().await;
        
        for change in changes {
            match change {
                FileChange::Create { path, content } |
                FileChange::Modify { path, content } => {
                    files.insert(path.clone(), content.clone());
                }
                FileChange::Delete { path } => {
                    files.remove(path);
                }
            }
        }
        
        Ok(())
    }
    
    pub async fn read_file(&self, path: &PathBuf) -> Option<String> {
        let files = self.files.read().await;
        files.get(path).cloned()
    }
}

pub struct ExecutorPool {
    router: Arc<dyn ModelRouter>,
    validator: Arc<dyn Validator>,
    max_concurrent: usize,
    workspace: Arc<WorkspaceState>,
}

impl ExecutorPool {
    pub fn new(
        router: Arc<dyn ModelRouter>,
        validator: Arc<dyn Validator>,
        max_concurrent: usize,
    ) -> Self {
        Self {
            router,
            validator,
            max_concurrent,
            workspace: WorkspaceState::new(),
        }
    }
    
    pub async fn execute_graph(&self, graph: TaskGraph) -> Result<Vec<TaskResult>> {
        if graph.has_cycles() {
            return Err(Error::CyclicDependency);
        }
        
        let mut completed = HashSet::new();
        let mut results = Vec::new();
        let mut join_set = JoinSet::new();
        let semaphore = Arc::new(Semaphore::new(self.max_concurrent));
        
        loop {
            let ready = graph.ready_tasks(&completed);
            
            if ready.is_empty() && join_set.is_empty() {
                break;
            }
            
            for task in ready {
                if join_set.len() >= self.max_concurrent {
                    break;
                }
                
                let router = self.router.clone();
                let validator = self.validator.clone();
                let workspace = self.workspace.clone();
                let permit = semaphore.clone().acquire_owned().await?;
                
                join_set.spawn(async move {
                    let result = Self::execute_task(
                        task,
                        router,
                        validator,
                        workspace,
                    ).await;
                    drop(permit);
                    result
                });
            }
            
            if let Some(result) = join_set.join_next().await {
                let task_result = result??;
                completed.insert(task_result.task_id);
                results.push(task_result);
            }
        }
        
        Ok(results)
    }
    
    async fn execute_task(
        task: Task,
        router: Arc<dyn ModelRouter>,
        validator: Arc<dyn Validator>,
        workspace: Arc<WorkspaceState>,
    ) -> Result<TaskResult> {
        const MAX_RETRIES: usize = 3;
        let mut attempt = 0;
        let mut current_tier = router.route(&task).await?.tier;
        
        loop {
            attempt += 1;
            
            let response = Self::execute_on_tier(&task, &current_tier).await?;
            let validation = validator.validate(&response, &task).await?;
            
            if validation.passed {
                if let Some(changes) = Self::extract_changes(&response) {
                    workspace.apply_changes(&changes).await?;
                }
                
                return Ok(TaskResult {
                    task_id: task.id,
                    response,
                    tier_used: current_tier,
                    validation,
                });
            }
            
            if attempt >= MAX_RETRIES {
                return Err(Error::MaxRetriesExceeded {
                    task_id: task.id,
                    validation,
                });
            }
            
            current_tier = current_tier
                .escalate()
                .ok_or(Error::NoHigherTierAvailable)?;
        }
    }
}

#[derive(Debug, Clone)]
pub enum FileChange {
    Create { path: PathBuf, content: String },
    Modify { path: PathBuf, content: String },
    Delete { path: PathBuf },
}
```

**Concurrency Safety:**
- ✅ `WorkspaceState` uses `RwLock` for synchronized file access
- ✅ `HashSet<TaskId>` for completion tracking (owned by `execute_graph`)
- ✅ `Semaphore` for explicit concurrency control
- ✅ Each task execution independent (no shared mutable state)
- ✅ Structured concurrency via `JoinSet`

---

### 4. Validation Layer

**File**: `merlin-routing/src/validator/mod.rs`

```rust
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub passed: bool,
    pub score: f64,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<String>,
    pub stages: Vec<StageResult>,
}

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub stage: ValidationStage,
    pub message: String,
    pub severity: Severity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationStage {
    Syntax,
    Build,
    Test,
    Lint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone)]
pub struct StageResult {
    pub stage: ValidationStage,
    pub passed: bool,
    pub duration_ms: u64,
    pub details: String,
}

#[async_trait]
pub trait Validator: Send + Sync {
    async fn validate(&self, response: &Response, task: &Task) -> Result<ValidationResult>;
    async fn quick_validate(&self, response: &Response) -> Result<bool>;
}

pub struct ValidationPipeline {
    stages: Vec<Arc<dyn ValidationStage>>,
    early_exit: bool,
}

#[async_trait]
impl Validator for ValidationPipeline {
    async fn validate(&self, response: &Response, task: &Task) -> Result<ValidationResult> {
        let mut result = ValidationResult {
            passed: true,
            score: 1.0,
            errors: Vec::new(),
            warnings: Vec::new(),
            stages: Vec::new(),
        };
        
        for stage in &self.stages {
            let stage_result = stage.validate(response, task).await?;
            
            result.stages.push(stage_result.clone());
            result.score *= stage_result.score;
            result.passed &= stage_result.passed;
            
            if !stage_result.passed && self.early_exit {
                break;
            }
        }
        
        Ok(result)
    }
    
    async fn quick_validate(&self, response: &Response) -> Result<bool> {
        if let Some(syntax_stage) = self.stages.first() {
            syntax_stage.quick_check(response).await
        } else {
            Ok(true)
        }
    }
}
```

---

## Module Communication Contracts

### Core Contracts

```rust
// Contract: TaskAnalyzer → TaskGraph
// Input: User request string
// Output: Vec<Task> with dependencies
// Guarantee: No cycles, all TaskIds unique

// Contract: ModelRouter → ExecutorPool
// Input: Task
// Output: RoutingDecision with ModelTier
// Guarantee: Tier is available, cost estimated

// Contract: ExecutorPool → WorkspaceState
// Input: FileChange array
// Output: Applied changes or error
// Guarantee: Atomic updates (all or nothing)

// Contract: Validator → ExecutorPool
// Input: Response, Task
// Output: ValidationResult
// Guarantee: Score 0.0-1.0, errors categorized
```

### Interface Boundaries

┌─────────────────────────────────────────────────────────┐
│                    Public API                            │
│  pub async fn execute_request(request: &str) -> Result  │
└─────────────────────────┬───────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────┐
│               Orchestrator (High-Level)                  │
│  - Coordinates all layers                                │
│  - No knowledge of model implementation details          │
└─────────────────────────┬───────────────────────────────┘
                          │
        ┌─────────────────┼─────────────────┐
        │                 │                 │
┌───────▼────────┐ ┌──────▼──────┐ ┌───────▼────────┐
│  TaskAnalyzer  │ │ ModelRouter │ │   Validator    │
│  (Trait)       │ │  (Trait)    │ │   (Trait)      │
└────────────────┘ └─────────────┘ └────────────────┘
        │                 │                 │
        └─────────────────┼─────────────────┘
                          │
                ┌─────────▼──────────┐
                │   ExecutorPool     │
                │  (Orchestrates)    │
                └─────────┬──────────┘
                          │
                ┌─────────▼──────────┐
                │  WorkspaceState    │
                │  (Synchronized)    │
                └────────────────────┘
```

**Key Points:**
- Each layer only knows about trait interfaces below it
- Implementation details hidden behind traits
- State synchronized at workspace boundary
- No cross-layer coupling

---

## Concurrency Patterns

### Pattern 1: Read-Heavy Workspace Access

```rust
// Multiple tasks reading same file concurrently
pub async fn concurrent_reads(workspace: Arc<WorkspaceState>, path: &PathBuf) {
    // ✅ Multiple readers allowed
    let content1 = workspace.read_file(path).await;
    let content2 = workspace.read_file(path).await;
    // Both execute concurrently
}
```

### Pattern 2: Exclusive Write Access

```rust
// Single task writing files
pub async fn exclusive_write(workspace: Arc<WorkspaceState>) {
    // ✅ Exclusive access during write
    workspace.apply_changes(&[
        FileChange::Modify { path, content }
    ]).await?;
    // Write lock released after await
}
```

### Pattern 3: Task Independence

```rust
// Tasks operate on separate data
pub async fn independent_tasks(task1: Task, task2: Task) {
    // ✅ No shared mutable state between tasks
    tokio::join!(
        execute_task(task1),
        execute_task(task2),
    );
    // Both execute in parallel safely
}
```

### Pattern 4: Dependency Synchronization

```rust
// Task B depends on Task A
pub async fn dependent_tasks(graph: TaskGraph) {
    let mut completed = HashSet::new();
    
    // ✅ Task B only starts after Task A completes
    loop {
        let ready = graph.ready_tasks(&completed);
        // ready_tasks returns B only after A in completed set
    }
}
```

---

## Error Handling Strategy

### Error Categories

```rust
#[derive(Debug, Error)]
pub enum RoutingError {
    // Retryable errors
    #[error("Provider temporarily unavailable: {0}")]
    ProviderUnavailable(String),
    
    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),
    
    #[error("Timeout after {0}ms")]
    Timeout(u64),
    
    // Non-retryable errors
    #[error("Cyclic dependency in task graph")]
    CyclicDependency,
    
    #[error("Invalid task configuration: {0}")]
    InvalidTask(String),
    
    #[error("No available tier for task")]
    NoAvailableTier,
    
    // Escalation errors
    #[error("Max retries exceeded for task {task_id}")]
    MaxRetriesExceeded {
        task_id: TaskId,
        validation: ValidationResult,
    },
    
    #[error("No higher tier available for escalation")]
    NoHigherTierAvailable,
}

impl RoutingError {
    pub const fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::ProviderUnavailable(_) |
            Self::RateLimitExceeded(_) |
            Self::Timeout(_)
        )
    }
    
    pub const fn can_escalate(&self) -> bool {
        matches!(
            self,
            Self::MaxRetriesExceeded { .. }
        )
    }
}
```

### Error Propagation

```rust
// Errors bubble up with context
pub async fn execute_with_context(task: Task) -> Result<TaskResult> {
    router.route(&task).await
        .map_err(|e| RoutingError::from(e).with_context("routing failed"))?;
        
    executor.execute(&task).await
        .map_err(|e| RoutingError::from(e).with_context("execution failed"))?;
}
```

---

## Testing Strategy

### Unit Tests (Per Component)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_task_graph_ready_tasks() {
        let task_a = Task { id: TaskId::new(), dependencies: vec![], ..};
        let task_b = Task { id: TaskId::new(), dependencies: vec![task_a.id], ..};
        
        let graph = TaskGraph::from_tasks(vec![task_a.clone(), task_b.clone()]);
        let mut completed = HashSet::new();
        
        // Initially only A is ready
        let ready = graph.ready_tasks(&completed);
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, task_a.id);
        
        // After A completes, B is ready
        completed.insert(task_a.id);
        let ready = graph.ready_tasks(&completed);
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, task_b.id);
    }
    
    #[tokio::test]
    async fn test_workspace_concurrent_reads() {
        let workspace = WorkspaceState::new();
        workspace.apply_changes(&[
            FileChange::Create {
                path: "test.rs".into(),
                content: "fn main() {}".to_string(),
            }
        ]).await.unwrap();
        
        // Concurrent reads should succeed
        let (content1, content2) = tokio::join!(
            workspace.read_file(&"test.rs".into()),
            workspace.read_file(&"test.rs".into()),
        );
        
        assert_eq!(content1, content2);
    }
}
```

### Integration Tests

```rust
#[cfg(test)]
mod integration_tests {
    // Test full pipeline with mock providers
    #[tokio::test]
    async fn test_parallel_execution_workflow() {
        let analyzer = MockTaskAnalyzer::new();
        let router = MockModelRouter::new();
        let validator = MockValidator::new();
        
        let executor = ExecutorPool::new(
            Arc::new(router),
            Arc::new(validator),
            2, // max_concurrent
        );
        
        let analysis = analyzer.analyze("Fix A and B").await?;
        let graph = TaskGraph::from_tasks(analysis.tasks);
        
        let results = executor.execute_graph(graph).await?;
        
        assert_eq!(results.len(), 2);
        assert!(results[0].validation.passed);
        assert!(results[1].validation.passed);
    }
}
```

---

## Performance Considerations

### Memory Management

```rust
// ✅ Use Arc for shared immutable data
let task = Arc::new(Task { .. });
let task_clone = task.clone(); // Cheap pointer clone

// ✅ Clone only when needed for owned data
let task = task.as_ref().clone(); // Full data clone

// ✅ Use references for temporary access
fn process_task(task: &Task) { .. }
```

### Lock Granularity

```rust
// ❌ Holding lock during expensive operation
{
    let mut files = workspace.files.write().await;
    let processed = expensive_operation(&files); // Lock held
    files.insert(path, processed);
}

// ✅ Release lock before expensive operation
let files_snapshot = {
    let files = workspace.files.read().await;
    files.clone()
};

let processed = expensive_operation(&files_snapshot);

{
    let mut files = workspace.files.write().await;
    files.insert(path, processed);
}
```

### Task Batching

```rust
// Process ready tasks in batches
let ready = graph.ready_tasks(&completed);

// ✅ Spawn up to max_concurrent at once
for task in ready.iter().take(max_concurrent) {
    join_set.spawn(execute_task(task));
}
```

---

## Configuration

### Runtime Configuration

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct RoutingConfig {
    pub max_concurrent_tasks: usize,
    pub enable_local_models: bool,
    pub enable_validation: bool,
    pub validation_timeout_seconds: u64,
    pub max_retries: usize,
    pub escalation_enabled: bool,
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: 4,
            enable_local_models: true,
            enable_validation: true,
            validation_timeout_seconds: 30,
            max_retries: 3,
            escalation_enabled: true,
        }
    }
}
```

### Model Tier Configuration

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct TierConfig {
    pub local_models: Vec<LocalModelConfig>,
    pub groq_api_key: Option<String>,
    pub openrouter_api_key: Option<String>,
    pub anthropic_api_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LocalModelConfig {
    pub name: String,
    pub model_path: PathBuf,
    pub max_tokens: usize,
}
```

---

## Migration Path

### Phase 1: Core Infrastructure
- Implement `Task`, `TaskGraph`, `WorkspaceState`
- Add `TaskAnalyzer` trait with simple implementation
- Add `ModelRouter` trait with complexity-based routing

### Phase 2: Execution Engine
- Implement `ExecutorPool` with parallel execution
- Add dependency tracking and synchronization
- Implement basic retry logic

### Phase 3: Validation Pipeline
- Add `Validator` trait and `ValidationPipeline`
- Implement syntax and build validation stages
- Integrate validation into execution loop

### Phase 4: Model Tiers
- Add local model integration (Ollama)
- Add Groq provider
- Implement tier escalation logic

### Phase 5: Isolation & Conflict Management
- Implement `FileLockManager` with RAII guards
- Add `TaskWorkspace` with transactional semantics
- Implement `IsolatedBuildEnv` for validation
- Add `ConflictAwareTaskGraph` for scheduling

### Phase 6: Advanced Features
- Add speculative execution
- Implement checklist-driven workflows
- Add cost tracking and optimization

---

## Complete Workflow Example

### Scenario: Parallel Refactor with Conflict Handling

**User Request:** "Refactor utils.rs and add tests in tests.rs"

#### Step 1: Analysis
```rust
let analysis = analyzer.analyze(request).await?;
// Result:
// Task A: Refactor utils.rs (modifies utils.rs)
// Task B: Add tests (modifies tests.rs, reads utils.rs)
// Dependency: B depends on A (needs refactored API)
```

#### Step 2: Conflict Detection
```rust
let conflict_graph = ConflictAwareTaskGraph::from_tasks(analysis.tasks);
// File access map:
// utils.rs -> [Task A (write), Task B (read)]
// tests.rs -> [Task B (write)]
// Conflict: A and B both access utils.rs
```

#### Step 3: Scheduled Execution
```rust
// Iteration 1:
ready = conflict_graph.ready_non_conflicting_tasks(&completed, &running);
// Result: [Task A] (no dependencies, no conflicts)
// Task B NOT ready (depends on A)

// Spawn Task A:
running = {Task A}
task_workspace_a = TaskWorkspace::new(Task A, [utils.rs], ...);
// Acquires write lock on utils.rs
```

#### Step 4: Task A Execution (Isolated)
```rust
// Execute in isolated workspace
response_a = execute_on_tier(&Task A, &task_workspace_a);
// Workspace state:
// - base_snapshot: {utils.rs: "old content"}
// - pending_changes: {utils.rs: "refactored content"}

// Validate in isolated build environment
build_env = IsolatedBuildEnv::new(workspace);
build_env.apply_changes(&response_a.changes);
validation_a = build_env.validate_build();
// ✅ Build passes
```

#### Step 5: Task A Commit
```rust
// Check for conflicts
conflicts = task_workspace_a.check_conflicts(global_workspace);
// No conflicts (no one else modified utils.rs)

// Commit atomically
task_workspace_a.commit(global_workspace);
// Global state updated:
// utils.rs: "refactored content"
// Lock released on utils.rs

completed = {Task A}
running = {}
```

#### Step 6: Task B Execution
```rust
// Iteration 2:
ready = conflict_graph.ready_non_conflicting_tasks(&completed, &running);
// Result: [Task B] (dependency satisfied, no running conflicts)

// Spawn Task B:
running = {Task B}
task_workspace_b = TaskWorkspace::new(Task B, [tests.rs], ...);
// Acquires write lock on tests.rs
// Acquires read lock on utils.rs (reads refactored version)

// Execute and validate
response_b = execute_on_tier(&Task B, &task_workspace_b);
validation_b = validate(&response_b);
// ✅ Tests pass

// Commit
task_workspace_b.commit(global_workspace);
completed = {Task A, Task B}
```

### Conflict Scenario: Concurrent Modifications

**User Request:** "Fix bug in parser.rs AND add feature using parser in compiler.rs"

```rust
// Both tasks want to modify parser.rs
let analysis = analyzer.analyze(request).await?;
// Task A: Fix bug in parser.rs (modifies parser.rs)
// Task B: Add feature (modifies parser.rs, compiler.rs)
// No explicit dependency, but file conflict

let conflict_graph = ConflictAwareTaskGraph::from_tasks(analysis.tasks);
// File access map:
// parser.rs -> [Task A (write), Task B (write)]
// compiler.rs -> [Task B (write)]

// Execution:
// Iteration 1:
ready = conflict_graph.ready_non_conflicting_tasks(&completed, &running);
// Result: [Task A] (picked first by scheduler)
// Task B NOT in ready (conflicts with running Task A)

spawn(Task A);
running = {Task A}

// Iteration 2 (while Task A still running):
ready = conflict_graph.ready_non_conflicting_tasks(&completed, &running);
// Result: [] (Task B conflicts with running Task A)
// Wait for Task A to complete

// Task A completes:
completed = {Task A}
running = {}

// Iteration 3:
ready = conflict_graph.ready_non_conflicting_tasks(&completed, &running);
// Result: [Task B] (no conflicts now)
spawn(Task B);

// Task B sees parser.rs changes from Task A in base_snapshot
// Task B builds on top of Task A's changes
// ✅ Sequential execution for conflicting files
```

---

## Summary

### Key Architectural Decisions

1. **Trait-based Design**: All major components behind traits for decoupling
2. **Immutable Tasks**: Tasks never mutate, simplifying concurrency
3. **Multi-Layer Isolation**: File locks, transactional workspaces, build isolation
4. **Conflict-Aware Scheduling**: Tasks with file conflicts run sequentially
5. **Optimistic Concurrency**: Snapshot-based isolation with conflict detection on commit
6. **RAII Lock Guards**: Automatic lock release on scope exit
7. **Structured Concurrency**: `JoinSet` ensures all tasks complete before proceeding
8. **Error Escalation**: Automatic tier escalation on validation failure
9. **Type-Safe State**: Rust type system prevents common concurrency bugs

### Concurrency Guarantees with Isolation

- ✅ **No data races**: All shared state behind `RwLock` + file locks
- ✅ **No file corruption**: Write locks exclusive, conflict detection on commit
- ✅ **No build interference**: Each task validates in isolated temp directory
- ✅ **Bounded parallelism**: `Semaphore` enforces limits
- ✅ **Dependency ordering**: `TaskGraph` ensures correct execution order
- ✅ **Conflict-free parallelism**: Scheduler prevents concurrent file access
- ✅ **Atomic updates**: All-or-nothing commits with rollback on conflict
- ✅ **Task isolation**: Each task has private workspace until commit

### Isolation Mode Trade-offs

| Mode | Parallelism | Safety | Overhead | Use Case |
|------|-------------|--------|----------|----------|
| **None** | Maximum | ❌ None | None | Single task only |
| **FileLocking** | High | ⚠️ Basic | Low | Independent files |
| **Transactional** | Medium | ✅ Strong | Medium | Most workflows |
| **FullIsolation** | Low | ✅✅ Maximum | High | Critical changes |

**Recommended**: `Transactional` for production (best balance)

### Protection Against Corruption

**File-Level Conflicts:**
- ✅ Write locks prevent simultaneous modifications
- ✅ Conflict detection before commit
- ✅ Automatic retry with updated base state

**Build State Conflicts:**
- ✅ Isolated temp directories for validation
- ✅ Tasks never see each other's broken builds
- ✅ Only validated changes committed to global state

**Cascading Changes:**
- ✅ Snapshot-based isolation captures base state
- ✅ Dependent tasks see committed changes
- ✅ Conflict detection catches stale modifications

### Extensibility Points

- New `RoutingStrategy` implementations
- New `ValidationStage` implementations
- Custom `TaskAnalyzer` for different analysis approaches
- Custom `IsolationMode` strategies
- Pluggable model providers via `ModelProvider` trait

---

**End of Architecture Document**

