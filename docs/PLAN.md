# Merlin: Next-Generation AI Coding Agent
## Development Roadmap

**Vision**: Build the most capable, efficient, and adaptable AI coding agent for managing large complex codebases

**Current Status**: Phases 1-3 Complete ✅ | 307+ tests passing
**Achievements**: Multi-Step Tasks | Model Routing | Context Intelligence | Citations | Dynamic Expansion

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

## TypeScript-Only TaskList Integration ✅ COMPLETE

**Achievement**: Pure TypeScript object flow with no JSON serialization

**Implemented**:
1. ✅ **TaskList Storage Pipeline**
   - `pending_task_list: Arc<Mutex<Option<TaskList>>>` in `AgentExecutor`
   - Store TaskList when TypeScript returns it (executor.rs:432)
   - Retrieve and pass via `TaskResult.task_list` field (executor.rs:246)

2. ✅ **TaskResult Integration**
   - Added `task_list: Option<TaskList>` field to `TaskResult`
   - Updated all 4 construction sites (executor.rs:253, 925, 1055; orchestrator.rs:258)
   - Changed `execute_typescript_code` to return `(AgentExecutionResult, Option<TaskList>)`

3. ✅ **Orchestrator Integration**
   - Check `result.task_list` directly instead of JSON parsing (orchestrator.rs:232)
   - TaskListExecutor executes steps sequentially with verification
   - Auto-fix on step failure using agent executor

4. ✅ **Clean Architecture**
   - Made `parse_task_list_from_value` private (internal use only)
   - No JSON serialization in the execution flow
   - Full workspace compilation passes

**Architecture**:
```
TypeScript Agent Returns TaskList Object
           ↓
execute_typescript_code parses serde_json::Value
           ↓
Stores in pending_task_list (Arc<Mutex<Option<TaskList>>>)
           ↓
execute retrieves and adds to TaskResult
           ↓
Orchestrator checks result.task_list (Option<TaskList>)
           ↓
TaskListExecutor executes steps sequentially
```

---

## Completed: Multi-Step Task Execution

**TaskList Structure**:
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

**Exit Commands**:
- Debug/Feature/Verify: `cargo check`
- Refactor: `cargo clippy -- -D warnings`
- Test: `cargo test`
- Custom: Agent can specify per-step

**Files**:
- `crates/merlin-agent/src/agent/task_list_executor.rs` - Executor (374 lines)
- `crates/merlin-agent/src/agent/command_runner.rs` - Command runner (167 lines)
- `crates/merlin-agent/src/orchestrator.rs` - Integration (lines 217-280)
- `prompts/typescript_agent.md` - Updated with TaskList examples

---

## Phase 2: Model Routing & Escalation ✅ MOSTLY COMPLETE

**Status**: Core functionality implemented, enhancements available

### 2.1 Model Selection Within Tiers ✅ IMPLEMENTED

**Implemented**:
- ✅ Multiple routing strategies with different model selection
- ✅ `ComplexityBasedStrategy`: Routes by task complexity (Trivial → Local Qwen, Simple → Groq Llama, Medium → Groq Qwen, Complex → Claude Sonnet)
- ✅ `LongContextStrategy`: Routes by context size (32k+ → Haiku, 100k+ → Sonnet)
- ✅ `CostOptimizationStrategy`: Cost-aware model selection across all tiers
- ✅ `QualityCriticalStrategy`: Premium models for high-priority tasks

**Files**:
- `crates/merlin-routing/src/router/strategies/complexity.rs`
- `crates/merlin-routing/src/router/strategies/context.rs`
- `crates/merlin-routing/src/router/strategies/cost.rs`
- `crates/merlin-routing/src/router/strategies/quality.rs`

### 2.2 Automatic Escalation ✅ COMPLETE

**Implemented**:
- ✅ Escalation chain: Local → Groq → Premium (orchestrator.rs:160)
- ✅ Automatic retry on failure (up to 3 retries, orchestrator.rs:117)
- ✅ Tier escalation with error tracking and UI events
- ✅ Graceful fallback when escalation not possible

**Files**:
- `crates/merlin-agent/src/orchestrator.rs` (lines 107-194)
- `crates/merlin-routing/src/router/mod.rs` (`escalate()` method, line 50)

### 2.3 Cost-Aware Routing ✅ IMPLEMENTED

**Implemented**:
- ✅ `CostOptimizationStrategy` with token-based routing
- ✅ Free tiers prioritized (Local, Groq) for smaller contexts
- ✅ Premium models only for large contexts or high priority
- ✅ Cost estimation per tier (tiers.rs:88-100)

**Potential Enhancements**:
- 🔄 Cost tracking and budget enforcement
- 🔄 Model-specific intent routing (DeepSeek for code, Claude for reasoning)
- 🔄 Runtime cost metrics collection

**Files**:
- `crates/merlin-routing/src/router/strategies/cost.rs`
- `crates/merlin-routing/src/router/tiers.rs`

---

## Phase 3: Response Quality & Context Intelligence ✅ COMPLETE

**Status**: All core features implemented

### 3.1 Intelligent Context Pruning ✅ COMPLETE

**Implemented**:
- ✅ `RelevanceScorer` - Keyword matching, file extension preferences, size optimization
- ✅ `DependencyGraph` - Rust-specific dependency extraction and transitive expansion
- ✅ `TokenBudgetAllocator` - Priority-based token distribution with min/max constraints
- ✅ Comprehensive test coverage (4 unit tests)

**Features**:
- Relevance scoring (0.0-1.0) based on keywords, extensions, size, recency markers
- Dependency graph building with use/mod statement parsing
- Transitive dependency expansion with configurable max depth
- Smart token allocation (30% priority reserve, score-based distribution)

**Files**:
- `crates/merlin-context/src/pruning.rs` - NEW (385 lines)

### 3.2 Context Citation Enforcement ✅ COMPLETE

**Implemented**:
- ✅ `Citation` - Parse and validate citations in format `file/path.rs:42-50`
- ✅ `CitationValidator` - Validate response citations against context files
- ✅ `CitationStatistics` - Track citation quality metrics
- ✅ Configurable enforcement (warnings vs errors)
- ✅ Comprehensive test coverage (5 unit tests)

**Features**:
- Citation parsing with line numbers and ranges
- Validation against available context files
- Minimum citation requirements
- Citation statistics (total, valid, invalid, unique files)
- Scoring system for validation quality

**Files**:
- `crates/merlin-agent/src/validator/citations.rs` - NEW (320 lines)

### 3.3 Dynamic Context Expansion ✅ COMPLETE

**Implemented**:
- ✅ `ContextRequestTool` - Agent tool for requesting additional files
- ✅ `ContextTracker` - Track files requested during conversation
- ✅ Glob pattern and exact file path support
- ✅ File size limits and validation
- ✅ TypeScript integration with proper signatures
- ✅ Comprehensive test coverage (3 unit tests)

**Features**:
- Request files by pattern (`**/*.rs`) or path (`src/lib.rs`)
- Automatic tracking of requested files (no duplicates)
- Configurable max files and file size limits
- Rich result data with file contents and metadata
- Proper error handling and messaging

**Files**:
- `crates/merlin-tooling/src/context_request.rs` - NEW (355 lines)

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

**Week 1-2**: Multi-Step Task Execution ✅ COMPLETE
1. ✅ Create `TaskListExecutor` with step-by-step execution
2. ✅ Add exit command runner and verification
3. ✅ Update TypeScript agent prompt with TaskList guidance
4. ✅ Integrate with orchestrator
5. ✅ Add TUI progress display (TaskStepStarted/Completed/Failed events)
6. ✅ Write 5+ E2E tests for workflows (11 structure tests + 12 integration tests)

**Week 3**: Model Routing ✅ COMPLETE
1. ✅ Model selection across all tiers (multiple strategies implemented)
2. ✅ Automatic escalation chain (Local → Groq → Premium)
3. ✅ Cost optimization strategy (token-based routing)

**Week 4-5**: Response Quality ✅ COMPLETE
1. ✅ Add context pruning (RelevanceScorer, DependencyGraph, TokenBudgetAllocator)
2. ✅ Implement citation validation (Citation, CitationValidator)
3. ✅ Add dynamic context requests (ContextRequestTool, ContextTracker)

**Success Criteria**: Agent executes multi-step workflows end-to-end with verification, displays progress, and completes tasks autonomously.
