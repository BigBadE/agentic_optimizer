# Merlin: Next-Generation AI Coding Agent
## Development Roadmap

**Vision**: Build the most capable, efficient, and adaptable AI coding agent for managing large complex codebases

**Current Status**: Phase 1 Complete | TaskList System Implemented | 307+ tests passing
**Next Phase**: Multi-Step Task Execution

---

## Table of Contents

1. [Recently Completed](#recently-completed)
2. [Current Focus: Multi-Step Task Execution](#current-focus-multi-step-task-execution)
3. [Phase 2: Model Routing & Escalation](#phase-2-model-routing--escalation)
4. [Phase 3: Response Quality & Context Intelligence](#phase-3-response-quality--context-intelligence)
5. [Architecture Overview](#architecture-overview)
6. [Success Metrics](#success-metrics)

---

## Recently Completed

### Phase 1: Action-Oriented Agent ✅ COMPLETE

**Achievement**: Agent now executes actions directly using tools

**Implemented**:
1. ✅ **Tool Execution System** - Agent outputs JSON tool calls, system executes them
2. ✅ **Executor Role Prompting** - Agent acts as autonomous executor, not advisor
3. ✅ **Multi-Step Workflows** - Workflow completion patterns in prompt
4. ✅ **Explicit Tool Examples** - 6 detailed examples with JSON format
5. ✅ **Debug Logging** - Comprehensive tracing for all agent output and tool execution
6. ✅ **TUI Task Ordering** - Fixed chronological task display (oldest first, newest last)

**Files Modified**:
- `prompts/coding_assistant.md` - Complete rewrite with executor role
- `crates/merlin-routing/src/agent/executor.rs` - Tool call extraction and execution
- `crates/merlin-routing/src/user_interface/` - Task ordering fixes

### TaskList System ✅ COMPLETE

**Achievement**: Structured task tracking with exit conditions

**Implemented**:
1. ✅ **TaskList Data Structures** (`crates/merlin-core/src/task_list.rs`)
   - `StepType` enum: Debug, Feature, Refactor, Verify, Test
   - `StepStatus` enum: Pending, InProgress, Completed, Failed, Skipped
   - `TaskStep` with id, description, verification, status, exit_command
   - `TaskList` with progress tracking and lifecycle methods

2. ✅ **Exit Conditions with Defaults**
   - Each step type has default verification command
   - Debug/Feature/Verify: `cargo check`
   - Refactor: `cargo clippy -- -D warnings`
   - Test: `cargo test`
   - Custom commands per-step via `exit_command` field

3. ✅ **Configuration System** (`crates/merlin-core/src/config.rs`)
   - `TaskListCommands` struct for configurable commands
   - Integrated into `RoutingConfig` as `task_list_commands`
   - Allows project-specific command customization

4. ✅ **TypeScript Agent Integration** (`prompts/typescript_agent.md`)
   - Documented `TaskList` and `TaskStep` interfaces
   - Examples showing default and custom exit commands
   - When to use TaskList vs simple string returns

5. ✅ **Comprehensive E2E Tests** (`crates/merlin-routing/tests/task_list_e2e.rs`)
   - 11 tests covering structure, lifecycle, progress tracking
   - 3 mock agent responses (simple, bug fix, refactoring workflows)
   - Exit command testing (default and custom)

### TypeScript Tool System ✅ COMPLETE

**Achievement**: Natural tool syntax for LLMs using TypeScript/JavaScript

**Implemented**:
- QuickJS-based runtime with sandboxing
- Full JavaScript support (control flow, error handling, functions)
- Tool registration and type definition generation
- Synchronous tool execution using Tokio runtime
- 15+ unit tests, 2 scenario tests passing

---

## Current Focus: Multi-Step Task Execution

**Timeline**: 2 weeks
**Priority**: CRITICAL - Enable agents to execute multi-step workflows using TaskLists

### What We Have Now

**TaskList Structure** (defined in Phase 1):
```typescript
interface TaskList {
    id: string;
    title: string;
    steps: TaskStep[];
    status: TaskListStatus;
}

interface TaskStep {
    id: string;
    step_type: "Debug" | "Feature" | "Refactor" | "Verify" | "Test";
    description: string;
    verification: string;
    status: "Pending" | "InProgress" | "Completed" | "Failed" | "Skipped";
    error?: string;
    result?: string;
    exit_command?: string;  // Custom command or null for default
}
```

**Exit Commands Already Configured**:
- Debug: `cargo check`
- Feature: `cargo check`
- Refactor: `cargo clippy -- -D warnings`
- Verify: `cargo check`
- Test: `cargo test`

### What We Need To Build

### 1. TaskList Executor

**Goal**: Execute TaskList step-by-step with verification

**Implementation**:

```rust
// crates/merlin-agent/src/task_list_executor.rs
pub struct TaskListExecutor {
    agent_executor: AgentExecutor,
    command_runner: CommandRunner,
}

impl TaskListExecutor {
    /// Execute a task list step-by-step
    pub async fn execute_task_list(
        &self,
        task_list: &mut TaskList,
        context: &Context,
    ) -> Result<TaskListResult> {
        for step in &mut task_list.steps {
            // Mark step as in progress
            step.start();

            // Execute the step using agent
            let step_result = self.execute_step(step, context).await?;

            // Run exit command verification
            let exit_cmd = step.get_exit_command();
            let verification = self.run_exit_command(exit_cmd).await?;

            // Update step based on verification result
            if verification.success {
                step.complete(Some(format!("✅ {}", verification.output)));
            } else {
                step.fail(format!("❌ Exit command failed: {}", verification.error));

                // Attempt auto-fix or escalate
                if self.attempt_fix(step, &verification).await? {
                    // Retry verification
                    continue;
                } else {
                    // Mark task list as failed
                    task_list.status = TaskListStatus::Failed;
                    return Ok(TaskListResult::Failed {
                        failed_step: step.id.clone()
                    });
                }
            }

            task_list.update_status();
        }

        Ok(TaskListResult::Success)
    }

    async fn execute_step(
        &self,
        step: &TaskStep,
        context: &Context,
    ) -> Result<AgentResponse> {
        // Generate prompt for this specific step
        let step_prompt = format!(
            "Execute step: {}\nType: {:?}\nVerification: {}",
            step.description,
            step.step_type,
            step.verification
        );

        // Execute with agent
        self.agent_executor.execute(&step_prompt, context).await
    }

    async fn run_exit_command(&self, command: &str) -> Result<CommandResult> {
        // Parse command (e.g., "cargo test --lib auth")
        let parts: Vec<&str> = command.split_whitespace().collect();
        let program = parts[0];
        let args = &parts[1..];

        // Execute command
        self.command_runner.run(program, args).await
    }

    async fn attempt_fix(
        &self,
        step: &TaskStep,
        verification: &CommandResult,
    ) -> Result<bool> {
        // Use agent to analyze failure and attempt fix
        let fix_prompt = format!(
            "Step failed: {}\nExit command: {}\nError: {}\n\nAnalyze and fix the issue.",
            step.description,
            step.get_exit_command(),
            verification.error
        );

        let fix_result = self.agent_executor.execute(&fix_prompt, &Context::empty()).await?;

        // Check if fix was successful by re-running exit command
        let recheck = self.run_exit_command(step.get_exit_command()).await?;
        Ok(recheck.success)
    }
}
```

**Files to Create**:
- `crates/merlin-agent/src/task_list_executor.rs` - Main executor
- `crates/merlin-agent/src/command_runner.rs` - Command execution utility

### 2. Agent Prompt Updates

**Goal**: Guide agent to create and execute TaskLists

**Updates to `prompts/typescript_agent.md`**:

```markdown
# TASK EXECUTION MODES

You have two execution modes:

## 1. Simple Task Mode (return string)
For single-step operations, return a string result:
```typescript
async function agent_code(): Promise<string> {
    const result = await bash("ls -la");
    return result.stdout;
}
```

## 2. TaskList Mode (return TaskList)
For multi-step workflows, return a TaskList plan:
```typescript
async function agent_code(): Promise<TaskList> {
    return {
        id: "fix_bug_123",
        title: "Fix authentication timeout bug",
        steps: [
            {
                id: "step_1",
                step_type: "Debug",
                description: "Read auth.rs to understand implementation",
                verification: "File loads and code structure is clear",
                status: "Pending",
                error: null,
                result: null,
                exit_command: null  // Uses default: cargo check
            },
            {
                id: "step_2",
                step_type: "Feature",
                description: "Add timeout configuration to AuthConfig",
                verification: "Code compiles without errors",
                status: "Pending",
                error: null,
                result: null,
                exit_command: null
            },
            {
                id: "step_3",
                step_type: "Test",
                description: "Run authentication tests",
                verification: "All tests pass",
                status: "Pending",
                error: null,
                result: null,
                exit_command: "cargo test --lib auth"  // Custom command
            }
        ],
        status: "NotStarted"
    };
}
```

## WHEN TO USE EACH MODE

Use **Simple Mode** for:
- Single tool calls (read file, list files, run command)
- Simple queries (what does X do?)
- Quick operations (count lines, find pattern)

Use **TaskList Mode** for:
- Bug fixes (Debug → Feature → Verify → Test)
- New features (Feature → Verify → Test)
- Refactoring (Refactor → Verify → Test)
- Any operation requiring multiple verification steps

## EXIT COMMAND BEHAVIOR

After you return a TaskList:
1. System executes each step in order
2. After each step, runs the exit_command
3. If exit_command succeeds (exit code 0), moves to next step
4. If exit_command fails, agent is called to fix the issue
5. Process continues until all steps complete or failure

**Exit command defaults** (set exit_command: null to use):
- Debug: `cargo check`
- Feature: `cargo check`
- Refactor: `cargo clippy -- -D warnings`
- Verify: `cargo check`
- Test: `cargo test`

**Custom exit commands** (set exit_command: "your command"):
- Specific module tests: `cargo test --lib module_name`
- Integration tests: `cargo test --test test_name`
- Custom validation: `./scripts/validate.sh`
```

### 3. Integration with Orchestrator

**Goal**: Wire TaskList execution into main orchestrator

**Updates to `crates/merlin-routing/src/orchestrator.rs`**:

```rust
pub async fn execute_task(&self, task: &Task) -> Result<TaskResult> {
    // Get agent response
    let response = self.agent_executor.execute(task).await?;

    // Check if response is a TaskList
    if let Some(task_list) = parse_task_list_from_response(&response)? {
        // Execute multi-step workflow
        let result = self.task_list_executor.execute_task_list(
            &mut task_list,
            &task.context
        ).await?;

        // Update UI with task list progress
        self.ui_channel.send(UiEvent::TaskListUpdate(task_list))?;

        return Ok(result.into());
    }

    // Regular single-step execution
    self.validate_and_return(response).await
}
```

### 4. TUI Integration

**Goal**: Display TaskList progress in TUI

**Updates to `crates/merlin-routing/src/user_interface/`**:

Add TaskList widget showing:
- Overall progress (3/5 steps complete)
- Current step being executed
- Step status icons (⏳ Pending, 🔄 In Progress, ✅ Complete, ❌ Failed)
- Exit command results
- Auto-scroll to current step

### 5. Testing Strategy

**E2E Tests to Add**:
1. Simple bug fix workflow (Debug → Feature → Test)
2. Refactoring workflow with clippy (Refactor → Verify → Test)
3. Multi-file feature (Feature → Feature → Verify → Test)
4. Failed step with auto-fix attempt
5. Custom exit commands

**Files to Test**:
- `tests/e2e/task_list_execution.rs` - Full workflow tests
- `tests/integration/task_list_executor.rs` - Unit tests for executor

---

## Phase 2: Model Routing & Escalation

**Timeline**: 1 week
**Priority**: HIGH - Enable cost-effective model selection

### 2.1 Fix Model Selection Within Tiers

**Problem**: Only one model used per tier, missing variety

**Implementation**:
1. Add model preferences to config per task type
2. Select appropriate OpenRouter model based on task intent
3. Route coding tasks to DeepSeek/Qwen, reasoning to Claude

**Files**:
- `crates/merlin-routing/src/router/tiers.rs`
- `crates/merlin-providers/src/openrouter.rs`
- `merlin.toml` - Add model preferences

### 2.2 Implement Automatic Escalation

**Problem**: No retry with higher tier on failure

**Implementation**:
1. Add escalation chain: Local → Groq → Premium
2. Detect failures (validation errors, tool errors, timeout)
3. Retry with next tier automatically
4. Track escalation metrics

**Files**:
- `crates/merlin-routing/src/orchestrator.rs`
- `crates/merlin-routing/src/validator/`

### 2.3 Enable Cost-Aware Routing

**Problem**: Simple tasks go to expensive models

**Implementation**:
1. Classify tasks by complexity (simple/medium/complex)
2. Route simple → Local, medium → Groq, complex → Premium
3. Track cost savings
4. Allow cost budget configuration

**Files**:
- `crates/merlin-routing/src/router/strategies/cost.rs`
- `crates/merlin-routing/src/analyzer/complexity.rs`

---

## Phase 3: Response Quality & Context Intelligence

**Timeline**: 2 weeks
**Priority**: MEDIUM - Optimize context and citations

### 3.1 Intelligent Context Pruning

**Problem**: Too many files in context, token waste

**Implementation**:
1. Add relevance scoring for selected files
2. Implement dependency graph expansion
3. Optimize token budget allocation
4. Track context effectiveness

**Files**:
- `crates/merlin-context/src/builder.rs`
- `crates/merlin-context/src/pruning.rs` (new)

### 3.2 Context Citation Enforcement

**Problem**: Agent doesn't cite sources

**Implementation**:
1. Add citation requirement to prompts
2. Validate citations in responses
3. Format: `file/path.rs:42-50`

**Files**:
- `prompts/coding_assistant.md`
- `crates/merlin-routing/src/validator/citations.rs` (new)

### 3.3 Dynamic Context Expansion

**Problem**: Sometimes need more context mid-task

**Implementation**:
1. Add `requestContext(pattern, reason)` tool
2. Allow agent to fetch additional files
3. Track requested files for conversation

**Files**:
- `crates/merlin-tooling/src/context_request.rs` (new)
- `crates/merlin-context/src/dynamic.rs` (new)

---

## Architecture Overview

### Current System (9 Crates)

**Core Infrastructure**:
- `merlin-core` - Types, traits, TaskList structures, config
- `merlin-context` - BM25 + embedding search
- `merlin-routing` - Multi-tier routing, validation, TUI
- `merlin-agent` - Agent execution, streaming
- `merlin-tooling` - Tool registry, TypeScript runtime

**Model Integration**:
- `merlin-providers` - Groq, OpenRouter, Anthropic, DeepSeek APIs
- `merlin-local` - Ollama integration
- `merlin-languages` - rust-analyzer backend

**CLI**:
- `merlin-cli` - Command-line interface, configuration

### Execution Pipeline

```
User Query → Analyzer → Router → Agent → TaskList? → Executor → Validator → Result
               ↓          ↓         ↓         ↓          ↓          ↓
           Complexity   Tier     TypeScript  Steps   Commands  Syntax
           Intent      Select    Runtime    Execute   Run      Build
           Scope      Escalate   Tools      Verify   Check     Test
                                Context    Progress  Pass     Lint
```

### Key Features Working

✅ TypeScript tool with QuickJS runtime
✅ Multi-tier model routing architecture
✅ Context fetching with BM25 + embeddings
✅ TUI with real-time updates
✅ TaskList data structures
✅ Exit condition system with defaults
✅ Response caching
✅ Workspace isolation
✅ Cost tracking and metrics

---

## Success Metrics

### Multi-Step Task Execution (Current Focus)

**Must Achieve**:
- TaskLists returned by agent for multi-step operations
- Steps executed in order with verification
- Exit commands run and validate success
- Failed steps trigger auto-fix attempts
- Progress displayed in TUI

**Measurement**:
- Test 10 multi-step workflows
- Success rate (target: >90%)
- Auto-fix success rate (target: >50%)
- User satisfaction with progress visibility

### Model Routing & Escalation

**Must Achieve**:
- 50% cost reduction through better tier selection
- Escalation working: Local → Groq → Premium
- Multiple models used per tier

**Measurement**:
- Average cost per request
- Escalation frequency (target: <15%)
- Model variety (target: 5+ models)

### Response Quality

**Must Achieve**:
- 80% of responses cite sources
- Context files reduced from 18 to 5-8
- Dynamic context requests working

**Measurement**:
- Citation rate
- Average context size
- Context request usage

---

## Next Steps (Priority Order)

**Week 1-2**: Multi-Step Task Execution
1. Create `TaskListExecutor` with step-by-step execution
2. Add exit command runner and verification
3. Update TypeScript agent prompt with TaskList guidance
4. Integrate with orchestrator
5. Add TUI progress display
6. Write 5+ E2E tests for workflows

**Week 3**: Model Routing
1. Add model selection within Premium tier
2. Implement escalation chain
3. Fix cost optimization strategy

**Week 4-5**: Response Quality
1. Add context pruning
2. Implement citation validation
3. Add dynamic context requests

**Success Criteria**: Agent executes multi-step workflows end-to-end with verification, displays progress, and completes tasks autonomously.
