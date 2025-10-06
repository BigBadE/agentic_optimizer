# Agentic Coding System - Design Document

**Version**: 1.0  
**Date**: 2025-10-05  
**Status**: Planning

---

## Executive Summary

This document outlines the transformation of Merlin from a simple task-based LLM router into a full-featured agentic coding assistant with:
- **Streaming execution** with real-time progress updates
- **Tool system** for file operations, command execution, and code analysis
- **Context accumulation** across task boundaries
- **Adaptive planning** that adjusts based on findings
- **Hierarchical task spawning** for complex workflows

---

## Current System Analysis

### Architecture Overview

```
User Input → Analyzer → Orchestrator → Executor → LLM → Result
                ↓
         TaskDecomposer
         (rigid 3-task split)
```

### Current Flow

1. **User submits**: "Create authentication module"
2. **IntentExtractor**: Identifies `Action::Create`
3. **TaskDecomposer**: Creates 3 tasks:
   - "Design structure: Create authentication module"
   - "Implement: Create authentication module"
   - "Add tests: Create authentication module"
4. **Orchestrator**: Executes sequentially
5. **Each task**: Gets same initial request, no context from previous
6. **Result**: Text-only responses, no file changes, no tool use

### Critical Limitations

| Issue | Impact | Priority |
|-------|--------|----------|
| No tool use | Can't read/write files, run commands | **Critical** |
| No streaming | User waits in darkness | **High** |
| No context flow | Each task starts from scratch | **Critical** |
| Rigid decomposition | Always 3 tasks regardless of complexity | **Medium** |
| No observability | Can't see reasoning or steps | **High** |
| Sequential only | Wastes time on independent tasks | **Low** |

---

## New System Architecture

### Component Hierarchy

```
┌─────────────────────────────────────────────────────────────────┐
│                         User Interface (TUI)                    │
│  - Task tree view                                               │
│  - Real-time step streaming                                     │
│  - Tool call visualization                                      │
│  - Context inspector                                            │
└────────────────────────┬────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────────────┐
│                    Orchestrator (Enhanced)                      │
│  - Manages execution lifecycle                                  │
│  - Coordinates streaming                                        │
│  - Accumulates context                                          │
│  - Handles tool execution                                       │
└────────────────────────┬────────────────────────────────────────┘
                         ↓
         ┌───────────────┴───────────────┐
         ↓                               ↓
┌──────────────────┐          ┌──────────────────┐
│  Agent Executor  │          │   Tool System    │
│  - Streaming     │ ←──────→ │  - File ops      │
│  - Step tracking │          │  - Commands      │
│  - Context mgmt  │          │  - Code analysis │
└──────────────────┘          └──────────────────┘
         ↓
┌──────────────────┐
│   LLM Provider   │
│  - Tool calling  │
│  - Streaming     │
└──────────────────┘
```

---

## Module Structure

### New Modules to Create

#### 1. `crates/merlin-routing/src/agent/`

**Purpose**: Core agentic execution logic

**Files**:
- `mod.rs` - Module exports
- `executor.rs` - Streaming task executor
- `step.rs` - Task step definitions
- `context.rs` - Execution context accumulation
- `planner.rs` - Adaptive planning logic

**Integration Point**: Called by `orchestrator.rs`

#### 2. `crates/merlin-routing/src/tools/`

**Purpose**: Tool system for agent capabilities

**Files**:
- `mod.rs` - Tool trait and registry
- `file_ops.rs` - Read/write/list files
- `command.rs` - Execute shell commands
- `code_analysis.rs` - Parse AST, search code
- `git.rs` - Git operations
- `test_runner.rs` - Run tests

**Integration Point**: Used by `agent/executor.rs`

#### 3. `crates/merlin-routing/src/streaming/`

**Purpose**: Streaming infrastructure

**Files**:
- `mod.rs` - Streaming types
- `channel.rs` - Event channels
- `buffer.rs` - Stream buffering

**Integration Point**: Used by `agent/executor.rs` and `ui/mod.rs`

### Modified Modules

#### 1. `crates/merlin-routing/src/types.rs`

**Changes**:
```rust
// Add new types
pub struct TaskStep {
    pub id: StepId,
    pub task_id: TaskId,
    pub step_type: StepType,
    pub timestamp: Instant,
    pub content: String,
}

pub enum StepType {
    Thinking,
    ToolCall { tool: String, args: Value },
    ToolResult { tool: String, result: Value },
    Output,
    SubtaskSpawned { child_id: TaskId },
}

pub struct ExecutionContext {
    pub original_request: String,
    pub files_read: HashMap<PathBuf, String>,
    pub files_written: HashMap<PathBuf, String>,
    pub commands_run: Vec<CommandExecution>,
    pub findings: Vec<String>,
    pub errors: Vec<String>,
    pub parent_results: Vec<TaskResult>,
}

pub struct CommandExecution {
    pub command: String,
    pub output: String,
    pub exit_code: i32,
    pub timestamp: Instant,
}

// Extend Task
pub struct Task {
    // ... existing fields ...
    pub parent_id: Option<TaskId>,
    pub can_spawn_subtasks: bool,
    pub execution_context: Option<ExecutionContext>,
}
```

#### 2. `crates/merlin-routing/src/ui/mod.rs`

**Changes**:
```rust
// Add new UI events
pub enum UiEvent {
    // ... existing events ...
    
    // New streaming events
    TaskStepStarted { task_id: TaskId, step: TaskStep },
    TaskStepCompleted { task_id: TaskId, step: TaskStep },
    ToolCallStarted { task_id: TaskId, tool: String, args: Value },
    ToolCallCompleted { task_id: TaskId, tool: String, result: Value },
    ThinkingUpdate { task_id: TaskId, content: String },
    SubtaskSpawned { parent_id: TaskId, child_id: TaskId, description: String },
    ContextUpdated { task_id: TaskId, context: ExecutionContext },
}

// Extend TaskDisplay
struct TaskDisplay {
    // ... existing fields ...
    steps: Vec<TaskStep>,
    tool_calls: Vec<ToolCall>,
    context: Option<ExecutionContext>,
}
```

**Rendering Changes**:
- Add expandable step list in output area
- Show tool calls with args/results
- Display context inspector panel
- Add tree view for hierarchical tasks

#### 3. `crates/merlin-routing/src/orchestrator.rs`

**Changes**:
```rust
impl RoutingOrchestrator {
    // New method for streaming execution
    pub async fn execute_task_streaming(
        &self,
        task: Task,
        ui_channel: UiChannel,
    ) -> Result<TaskResult> {
        // Create agent executor
        let executor = AgentExecutor::new(
            self.router.clone(),
            self.validator.clone(),
            self.tool_registry.clone(),
        );
        
        // Execute with streaming
        executor.execute_streaming(task, ui_channel).await
    }
    
    // New method for context-aware execution
    pub async fn execute_with_context(
        &self,
        task: Task,
        context: ExecutionContext,
        ui_channel: UiChannel,
    ) -> Result<TaskResult> {
        let task_with_context = task.with_context(context);
        self.execute_task_streaming(task_with_context, ui_channel).await
    }
}
```

---

## Implementation Plan

### Phase 1: Foundation (Week 1) ✅ **COMPLETED**

**Goal**: Basic tool system and streaming infrastructure

**Tasks**:
1. ✅ Create `tools/` module structure
2. ✅ Implement core tools:
   - ✅ `ReadFileTool` - Read file contents with security checks
   - ✅ `WriteFileTool` - Write/create files with directory creation
   - ✅ `ListFilesTool` - List directory contents
   - ✅ `RunCommandTool` - Execute whitelisted commands
3. ✅ Create `streaming/` module
4. ✅ Add streaming event types to `types.rs`
   - ✅ `ExecutionContext` - Context accumulation
   - ✅ `CommandExecution` - Command tracking
5. ✅ Update `UiEvent` enum with streaming events
   - ✅ `TaskStepStarted/Completed`
   - ✅ `ToolCallStarted/Completed`
   - ✅ `ThinkingUpdate`
   - ✅ `SubtaskSpawned`
6. ✅ Add placeholder event handlers in UI
7. ✅ Export new modules in `lib.rs`

**Deliverable**: ✅ Tools can be called, events can be streamed to UI

**Testing**: ✅ All 7 tests passing
```rust
✅ test_read_file_tool
✅ test_write_file_tool
✅ test_list_files_tool
✅ test_run_command_tool
✅ test_command_whitelist
✅ test_custom_whitelist
✅ test_security_path_traversal
```

**Implementation Notes**:
- Tools include comprehensive security checks (path traversal prevention)
- Command tool uses whitelist approach (default: cargo, git, rustc, rustfmt, clippy)
- Streaming events have placeholder UI handlers ready for Phase 2
- `Instant` fields use `#[serde(skip, default)]` for serialization compatibility

### Phase 2: Agent Executor (Week 2) ✅ **COMPLETED**

**Goal**: Streaming task execution with tool calling

**Tasks**:
1. ✅ Create `agent/` module structure
   - ✅ `mod.rs` - Module exports
   - ✅ `executor.rs` - AgentExecutor implementation
   - ✅ `step.rs` - StepTracker for managing steps
2. ✅ Implement `AgentExecutor`
   - ✅ Streaming execution method
   - ✅ Tool calling integration
   - ✅ Provider creation
   - ✅ Context building
3. ✅ Add step tracking (`TaskStep`)
   - ✅ StepTracker stores steps per task
   - ✅ Steps include: Thinking, ToolCall, ToolResult, Output
4. ✅ Integrate tool calling with LLM
   - ✅ Tool registry passed to executor
   - ✅ Tools added to query description
   - ✅ Tool execution with error handling
5. ✅ Stream steps to UI in real-time
   - ✅ TaskStepStarted/Completed events
   - ✅ ToolCallStarted/Completed events
   - ✅ ThinkingUpdate events
6. ✅ Update UI to display steps
   - ✅ TaskStepInfo structure for storing steps
   - ✅ Icon-based step display (💭 🔧 📝)
   - ✅ Proper event handlers for all streaming events
7. ✅ Update orchestrator
   - ✅ `execute_task_streaming()` method
   - ✅ Tool registry creation with workspace tools
   - ✅ AgentExecutor integration

**Deliverable**: ✅ Tasks execute with visible steps and tool calls

**Testing**: ✅ All 2 tests passing
```rust
✅ test_agent_executor_creation
✅ test_tool_registry_integration
```

**Implementation Notes**:
- AgentExecutor creates tool registry with all workspace tools
- Steps are tracked and displayed with visual icons
- Tool calls show "Calling tool: X" and "✓ Tool 'X' completed"
- Thinking steps show with 💭 icon
- All streaming events properly integrated with UI
- Legacy `execute_task()` method preserved for backward compatibility

**Example Flow**:
```
User: "Add logging to main.rs"

Step 1: Thinking
  → "Need to read current main.rs to understand structure"
  
Step 2: Tool Call
  → ReadFile("src/main.rs")
  → Result: [file contents]
  
Step 3: Thinking
  → "Will add env_logger crate and initialize in main()"
  
Step 4: Tool Call
  → WriteFile("Cargo.toml", [updated content])
  
Step 5: Tool Call
  → WriteFile("src/main.rs", [updated content])
  
Step 6: Tool Call
  → RunCommand("cargo check")
  → Result: "Compiling... Finished"
  
Step 7: Output
  → "Added env_logger with INFO level default"
```

### Phase 3: Context Accumulation (Week 3)

**Goal**: Tasks share context and learnings

**Tasks**:
1. Implement `ExecutionContext` in `agent/context.rs`
2. Accumulate findings across tasks
3. Pass context to subsequent tasks
4. Display context in UI
5. Add context inspector panel

**Deliverable**: Sequential tasks build on each other's work

**Example**:
```
Task 1: "Analyze authentication needs"
  → Findings: "No auth middleware, using actix-web 4.x"
  → Context: { framework: "actix-web", version: "4.x" }

Task 2: "Implement authentication" (receives Task 1 context)
  → Uses findings to choose compatible auth crate
  → Adds to context: { auth_crate: "actix-web-httpauth" }

Task 3: "Add tests" (receives Task 1 + 2 context)
  → Knows which framework and auth crate to test
```

### Phase 4: Adaptive Planning (Week 4)

**Goal**: Agent can replan based on findings

**Tasks**:
1. Implement `AdaptivePlanner` in `agent/planner.rs`
2. Add phase-based execution
3. Allow replanning between phases
4. Add UI for plan visualization
5. Support user approval of plans

**Deliverable**: Agent creates and adjusts plans dynamically

**Example**:
```
User: "Optimize database queries"

Initial Plan:
  Phase 1: Profile performance
  Phase 2: TBD (depends on findings)

After Phase 1:
  Findings: "N+1 query problem in user lookup"
  
Replanned:
  Phase 2: Implement query batching
  Phase 3: Add eager loading
  Phase 4: Benchmark improvements
```

### Phase 5: Hierarchical Tasks (Week 5)

**Goal**: Tasks can spawn subtasks

**Tasks**:
1. Add parent-child relationships to `Task`
2. Implement subtask spawning in executor
3. Update UI for tree view
4. Handle dependency resolution
5. Add collapse/expand for task trees

**Deliverable**: Complex tasks decompose dynamically

**Example**:
```
Task: "Refactor authentication module"
  ├─ Subtask: "Extract auth logic to auth.rs"
  ├─ Subtask: "Update imports in main.rs"
  ├─ Subtask: "Update imports in api/mod.rs"
  └─ Subtask: "Migrate auth tests"
```

---

## Integration Points

### 1. Orchestrator → Agent Executor

**Location**: `orchestrator.rs:72` (in `execute_task`)

**Change**:
```rust
// OLD
pub async fn execute_task(&self, task: Task) -> Result<TaskResult> {
    let provider = self.create_provider(&decision.tier)?;
    let response = provider.generate(&query, &context).await?;
    // ...
}

// NEW
pub async fn execute_task(&self, task: Task, ui_channel: UiChannel) -> Result<TaskResult> {
    let executor = AgentExecutor::new(
        self.router.clone(),
        self.tool_registry.clone(),
    );
    executor.execute_streaming(task, ui_channel).await
}
```

### 2. CLI → Orchestrator

**Location**: `merlin-cli/src/main.rs:545`

**Change**:
```rust
// OLD
match orchestrator_clone.execute_tasks(analysis.tasks).await {
    Ok(results) => { /* ... */ }
}

// NEW
let mut context = ExecutionContext::new(user_input.clone());
for task in analysis.tasks {
    let result = orchestrator_clone
        .execute_with_context(task, context.clone(), ui_channel_clone.clone())
        .await?;
    
    // Accumulate context for next task
    context.add_result(result.clone());
    
    ui_channel_clone.completed(result.task_id, result);
}
```

### 3. UI → Event Handlers

**Location**: `ui/mod.rs:1107` (in `handle_event`)

**Add**:
```rust
UiEvent::TaskStepStarted { task_id, step } => {
    if let Some(task) = self.state.tasks.get_mut(&task_id) {
        task.steps.push(step);
    }
}

UiEvent::ToolCallStarted { task_id, tool, args } => {
    if let Some(task) = self.state.tasks.get_mut(&task_id) {
        task.tool_calls.push(ToolCall {
            tool: tool.clone(),
            args: args.clone(),
            result: None,
            timestamp: Instant::now(),
        });
    }
}

UiEvent::ToolCallCompleted { task_id, tool, result } => {
    if let Some(task) = self.state.tasks.get_mut(&task_id) {
        if let Some(call) = task.tool_calls.last_mut() {
            call.result = Some(result);
        }
    }
}
```

### 4. UI Rendering

**Location**: `ui/mod.rs:1417` (output rendering)

**Add**:
```rust
// Render task steps
if !task.steps.is_empty() {
    let steps_text: Vec<Line> = task.steps.iter().map(|step| {
        match &step.step_type {
            StepType::Thinking => {
                Line::from(Span::styled(
                    format!("💭 {}", step.content),
                    Style::default().fg(Color::Gray)
                ))
            }
            StepType::ToolCall { tool, .. } => {
                Line::from(Span::styled(
                    format!("🔧 {}", tool),
                    Style::default().fg(Color::Yellow)
                ))
            }
            StepType::Output => {
                Line::from(step.content.clone())
            }
            // ... other types
        }
    }).collect();
    
    // Render steps before output
}
```

---

## File Structure After Implementation

```
crates/merlin-routing/src/
├── agent/
│   ├── mod.rs              # NEW: Agent module exports
│   ├── executor.rs         # NEW: Streaming executor
│   ├── step.rs             # NEW: Step definitions
│   ├── context.rs          # NEW: Context accumulation
│   └── planner.rs          # NEW: Adaptive planning
├── tools/
│   ├── mod.rs              # NEW: Tool trait & registry
│   ├── file_ops.rs         # NEW: File operations
│   ├── command.rs          # NEW: Command execution
│   ├── code_analysis.rs    # NEW: Code parsing
│   ├── git.rs              # NEW: Git operations
│   └── test_runner.rs      # NEW: Test execution
├── streaming/
│   ├── mod.rs              # NEW: Streaming types
│   ├── channel.rs          # NEW: Event channels
│   └── buffer.rs           # NEW: Stream buffering
├── analyzer/               # EXISTING
├── executor/               # EXISTING
├── router/                 # EXISTING
├── validator/              # EXISTING
├── ui/
│   ├── mod.rs              # MODIFIED: Add streaming events
│   └── events.rs           # NEW: Event definitions
├── types.rs                # MODIFIED: Add new types
├── orchestrator.rs         # MODIFIED: Add streaming methods
└── lib.rs                  # MODIFIED: Export new modules
```

---

## Testing Strategy

### Unit Tests

**Tools**:
```rust
#[tokio::test]
async fn test_read_file_tool() { /* ... */ }

#[tokio::test]
async fn test_write_file_tool() { /* ... */ }

#[tokio::test]
async fn test_command_tool() { /* ... */ }
```

**Agent Executor**:
```rust
#[tokio::test]
async fn test_streaming_execution() { /* ... */ }

#[tokio::test]
async fn test_tool_calling() { /* ... */ }

#[tokio::test]
async fn test_context_accumulation() { /* ... */ }
```

### Integration Tests

**End-to-End**:
```rust
#[tokio::test]
async fn test_full_agentic_flow() {
    // User request → Planning → Execution → Tool use → Result
}

#[tokio::test]
async fn test_hierarchical_tasks() {
    // Task spawns subtasks, all complete successfully
}
```

### Manual Testing Scenarios

1. **Simple file modification**: "Add a comment to main.rs"
2. **Multi-file change**: "Refactor auth module"
3. **Complex workflow**: "Add new feature with tests"
4. **Error handling**: "Fix syntax error in parser.rs"
5. **Adaptive planning**: "Optimize performance" (requires profiling first)

---

## Success Metrics

| Metric | Current | Target | Measurement |
|--------|---------|--------|-------------|
| User visibility | 0% (black box) | 100% (all steps visible) | Can see every tool call |
| Context retention | 0% (each task isolated) | 100% (full context flow) | Tasks reference previous findings |
| Tool usage | 0 tools | 8+ tools | File ops, commands, git, tests |
| Streaming | No | Yes | Real-time step updates |
| Adaptability | Rigid 3-task split | Dynamic planning | Plans adjust to findings |
| Intervention | None | Full | Can pause/guide at any step |

---

## Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| LLM doesn't support tool calling | **High** | Use function calling format, fallback to text parsing |
| Streaming adds latency | **Medium** | Buffer aggressively, async channels |
| Context grows too large | **Medium** | Implement context pruning, summarization |
| Tool execution security | **High** | Sandbox commands, whitelist operations |
| UI complexity increases | **Medium** | Progressive disclosure, collapsible sections |

---

## Open Questions

1. **Tool security**: How to sandbox command execution safely?
   - **Answer**: Use `std::process::Command` with restricted permissions, whitelist allowed commands

2. **Context size limits**: How to handle when context exceeds token limits?
   - **Answer**: Implement smart summarization, keep only relevant findings

3. **Streaming performance**: Will real-time updates slow down execution?
   - **Answer**: Use async channels with buffering, batch small updates

4. **User interruption**: How to cleanly stop mid-execution?
   - **Answer**: Use cancellation tokens, ensure graceful cleanup

5. **Tool failure handling**: What if a tool call fails?
   - **Answer**: Agent should see error, can retry or adjust plan

---

## Next Steps

1. **Review this document** with team
2. **Create GitHub issues** for each phase
3. **Set up project board** with milestones
4. **Begin Phase 1** implementation
5. **Weekly progress reviews**

---

## Appendix: Example Flows

### Example 1: Simple File Modification

```
User: "Add logging to main.rs"

[Planning Agent]
  Plan: Single task - add logging

[Task: Add Logging]
  Step 1: 💭 "Need to see current main.rs structure"
  Step 2: 🔧 ReadFile("src/main.rs")
          → Result: [current contents]
  Step 3: 💭 "Will add env_logger crate"
  Step 4: 🔧 ReadFile("Cargo.toml")
          → Result: [current dependencies]
  Step 5: 🔧 WriteFile("Cargo.toml", [with env_logger])
  Step 6: 🔧 WriteFile("src/main.rs", [with logger init])
  Step 7: 🔧 RunCommand("cargo check")
          → Result: "Finished dev [unoptimized]"
  Step 8: ✅ "Added env_logger with INFO level"

[Complete] ✓
```

### Example 2: Complex Refactoring

```
User: "Refactor authentication into separate module"

[Planning Agent]
  Phase 1: Analysis
  Phase 2: Extraction
  Phase 3: Integration
  Phase 4: Testing

[Phase 1: Analysis]
  Task: Analyze current auth code
    → 🔧 SearchCode("fn.*auth")
    → 🔧 ReadFile("src/main.rs")
    → 🔧 ReadFile("src/api/handlers.rs")
    → Findings: "Auth logic scattered across 3 files"

[Phase 2: Extraction]
  Task: Create auth module
    → 🔧 WriteFile("src/auth/mod.rs", [new module])
    → 🔧 WriteFile("src/auth/middleware.rs", [extracted logic])
    
  Task: Update main.rs
    → 🔧 ReadFile("src/main.rs")
    → 🔧 WriteFile("src/main.rs", [updated imports])

[Phase 3: Integration]
  Task: Update API handlers
    → 🔧 ReadFile("src/api/handlers.rs")
    → 🔧 WriteFile("src/api/handlers.rs", [use new module])
    
  Task: Verify compilation
    → 🔧 RunCommand("cargo check")
    → ✅ "No errors"

[Phase 4: Testing]
  Task: Migrate tests
    → 🔧 WriteFile("src/auth/tests.rs", [moved tests])
    → 🔧 RunCommand("cargo test auth")
    → ✅ "5 tests passed"

[Complete] ✓ Refactored auth into src/auth/ module
```

---

**End of Document**
