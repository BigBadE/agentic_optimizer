# Merlin: Next-Generation AI Coding Agent
## Development Roadmap - Agent Usability Focus

**Vision**: Build the most capable, efficient, and adaptable AI coding agent for managing large complex codebases

**Current Status**: Infrastructure Complete | 160 Rust files | 307 tests passing
**Critical Issues**: Agent behavior needs fundamental improvements for basic usability
**Next Phase**: Make the Agent Actually Usable

---

## Table of Contents

1. [Critical Issues](#critical-issues)
2. [Phase 1: Action-Oriented Agent (CRITICAL)](#phase-1-action-oriented-agent-critical)
3. [Phase 2: Model Routing & Escalation](#phase-2-model-routing--escalation)
4. [Phase 3: Task Management & Verification](#phase-3-task-management--verification)
5. [Phase 4: Continuation & Multi-Step Workflows](#phase-4-continuation--multi-step-workflows)
6. [Phase 5: Response Quality & Context Intelligence](#phase-5-response-quality--context-intelligence)
7. [Current Architecture](#current-architecture)
8. [Success Metrics](#success-metrics)

---

## Critical Issues

### What's Actually Broken (Verified)

**Priority 1 - Agent Behavior**:
1. **Agents explain instead of executing**
   - Agent outputs "You should do X" instead of calling tools to do X
   - Provides instructions instead of performing actions
   - Treats user as if they have access to tools

2. **Agents expect users to use tools**
   - "You can use readFile to..." instead of calling readFile themselves
   - Acts as an advisor rather than autonomous executor
   - Doesn't understand it has direct tool access

3. **Agents stop prematurely**
   - Completes first step and stops
   - Doesn't continue to verify, test, or complete workflow
   - No multi-step task execution

**Priority 2 - System Gaps**:
4. **No model routing between tiers**
   - OpenRouter models not being selected
   - Groq/Local tier selection not working
   - Escalation logic not functioning

5. **No enforced task management**
   - No structured task list tracking
   - No step verification before moving forward
   - No testing requirements after code changes

### What Actually Works

**Infrastructure** (all the plumbing is done):
- ✅ Multi-tier model routing architecture
- ✅ Context fetching with BM25 + embeddings
- ✅ Tool registry and execution
- ✅ TUI with real-time updates
- ✅ TypeScript tool for complex workflows
- ✅ Validation pipeline
- ✅ Response caching
- ✅ Workspace isolation

**The Problem**: Infrastructure is solid, agent prompting and behavior is broken

---

## Phase 1: Action-Oriented Agent (CRITICAL)
**Timeline**: 1 week
**Priority**: CRITICAL - Agent must execute actions, not explain them

### 1.1 Make Agent Execute Tools Directly ✅ COMPLETED

**Problem**: Agent outputs TypeScript code or explanations instead of executing tools

**Root Cause (Verified)**:
- Checked `prompts/coding_assistant.md` and `crates/merlin-agent/src/executor.rs:186-271`
- Agent must output JSON in format: `{"tool": "tool_name", "params": {...}}`
- Prompt examples showed pseudocode like `[calls readFile(...)]` instead of actual JSON format
- Executor looks for JSON in response (lines 250-271) and extracts tool calls
- No explicit anti-patterns showing what NOT to do
- Lacks examples using correct JSON format

**Solution Implemented**:
- Rewrote prompt with JSON format in ALL examples
- Added WRONG vs RIGHT examples showing JSON format
- Added explicit "CRITICAL REMINDERS" section emphasizing JSON format
- Clarified one tool call per response (system executes then returns control)

**Key Changes**:
```markdown
# CRITICAL: You are an AUTONOMOUS EXECUTOR, not an advisor

YOU HAVE DIRECT TOOL ACCESS. When the user asks you to do something:
- ❌ NEVER say "You can use readFile..."
- ❌ NEVER say "You should modify..."
- ❌ NEVER provide instructions for the user
- ✅ ALWAYS call tools directly yourself
- ✅ ALWAYS perform the action immediately

WRONG (Advisory):
User: "Read src/main.rs"
Agent: "You can use the readFile tool to read src/main.rs"

RIGHT (Executor):
User: "Read src/main.rs"
Agent: [calls readFile("src/main.rs")]
Agent: "Here are the contents of src/main.rs: ..."

WRONG (Instructional):
User: "Fix the bug in foo.rs"
Agent: "You should open foo.rs and change line 42 from X to Y"

RIGHT (Executor):
User: "Fix the bug in foo.rs"
Agent: [calls readFile("foo.rs")]
Agent: [calls writeFile("foo.rs", fixed_content)]
Agent: "Fixed the bug by changing line 42 from X to Y"
```

**Implementation**:
1. Update `prompts/coding_assistant.md` with executor role emphasis
2. Add explicit anti-patterns (what NOT to do)
3. Add few-shot examples of correct executor behavior
4. Remove any advisory language from existing prompt

**Testing**:
- Test queries: "Read X", "Modify Y", "Run Z"
- Success = tool called immediately, no instructions given
- Failure = any response containing "you can", "you should"

**Files**:
- `prompts/coding_assistant.md` - Rewrite executor section

**Files Modified**:
- `prompts/coding_assistant.md` - Complete rewrite with JSON examples

**Testing Status**: Ready for manual testing with agent

---

### 1.2 Enforce Multi-Step Completion ✅ COMPLETED

**Problem**: Agent stops after first step instead of continuing

**Solution Implemented**:
- Added "WORKFLOW COMPLETION - CRITICAL" section to prompt
- Defined required patterns for common operations
- Emphasized completing ENTIRE workflow with verification

**Files Modified**:
- `prompts/coding_assistant.md:94-116` - Workflow completion section

---

### 1.3 Add Explicit Tool Examples ✅ COMPLETED

**Problem**: Agent doesn't understand when/how to use tools

**Solution Implemented**:
- Added 6 detailed examples in correct JSON format
- Showed simple tool calls vs TypeScript tool usage
- Included full workflows with verification steps

**Files Modified**:
- `prompts/coding_assistant.md:124-374` - Tool usage examples

---

## Phase 1 Status: COMPLETE ✅

**Issues Fixed**:

### Issue 1: Agent Output Format ✅
- **Problem**: Agent output markdown code blocks instead of JSON tool calls
- **Root Cause**: Tool instructions buried at end of prompt, models ignored them
- **Fix**: Moved "YOU ARE A TOOL-CALLING AGENT" to top of prompt with explicit anti-patterns
- **Files**: `prompts/coding_assistant.md:21-45, 390-422`
- **Result**: Agent now outputs JSON tool calls correctly

### Issue 2: Tool Calls Not Executing ✅
- **Problem**: Agent output correct JSON tool calls, but nothing executed
- **Root Cause (CRITICAL)**: `execute_with_streaming()` had hardcoded empty vector for tool calls (line 266: `let tool_calls: Vec<(String, Value)> = vec![];`)
- **Comment in code**: "simulated for now" and "In a real implementation, this would parse..."
- **Fix**: Implemented `extract_tool_calls()` method to parse JSON from LLM response
- **Parses**: Both ```json blocks and raw JSON `{...}` format
- **Extracts**: `{"tool": "tool_name", "params": {...}}` structure
- **Files**: `crates/merlin-routing/src/agent/executor.rs:265, 345-404`
- **Result**: Tools now execute when agent outputs JSON

### Issue 3: Wrong Tool Names in Prompt ✅
- **Problem**: Prompt examples showed wrong tool names
- **Fix**: Updated all examples with correct tool names (`bash`, `show`, `edit`, `list`, `delete`, `execute_typescript`)
- **Files**: `prompts/coding_assistant.md:197-239`
- **Result**: Agent uses correct tool names

### Issue 4: Task List Order & Duplicate Logic ✅
- **Problem**: Newer tasks appeared at top, older tasks at bottom (reverse chronological) on first load
- **Root Causes (Multiple)**:
  1. `rebuild_order()` used `Reverse(*time)` sorting (lines 140, 286) ✅ Fixed
  2. **Renderer ignored `task_order` entirely!** (line 431) ✅ Fixed
     - Renderer called `iter_tasks()` which returns unordered HashMap iterator
     - Then re-sorted by start_time, but HashMap order is undefined
     - Completely bypassed TaskManager's maintained order
  3. **Duplicate sorting logic everywhere** ✅ Fixed
     - `build_visible_task_list()` used `iter_tasks()` + sort
     - Task deletion logic used `iter_tasks()` + sort
     - Renderer used `iter_tasks()` + sort
     - All ignored the pre-maintained `task_order`
  4. **Unused `get_visible_tasks()` method** ✅ Fixed
     - Method was never used in production code, only tests
     - Tests duplicated the collapse filtering logic
- **Fixes**:
  1. Removed `Reverse()` from TaskManager sorting (ascending = oldest first, newest last)
  2. **Changed renderer to use `task_order()`** instead of `iter_tasks()`
  3. **Changed `build_visible_task_list()` to use `task_order()`** (app.rs:769-795)
  4. **Changed task deletion to use `task_order()`** (app.rs:944-954)
  5. **Removed `get_visible_tasks()` entirely** - unused in production code
  6. **Made `is_hidden_by_collapse()` public** for tests to use directly (task_manager.rs:297)
  7. **Updated all tests** to use `task_order()` + `is_hidden_by_collapse()` instead of `get_visible_tasks()`
  8. Removed all duplicate sorting - single source of truth is `task_order`
- **Files**:
  - `crates/merlin-routing/src/user_interface/task_manager.rs:140, 286, 297` - Fixed sorting, removed get_visible_tasks, made is_hidden_by_collapse public
  - `crates/merlin-routing/src/user_interface/renderer.rs:430-448` - Use task_order
  - `crates/merlin-routing/src/user_interface/app.rs:769-795, 944-954` - Use task_order everywhere
  - `crates/merlin-routing/tests/unit/tasks/task_manager_tests.rs` - Updated all tests to use task_order + is_hidden_by_collapse
  - `crates/merlin-routing/tests/unit/ui/tui_edge_cases_tests.rs` - Updated all tests to use task_order + is_hidden_by_collapse
  - `crates/merlin-routing/tests/scenario_runner.rs:775-807, 879-913, 956-961` - Updated test helpers to use task_order + is_hidden_by_collapse
- **Tests Added/Updated**:
  - `test_task_order_after_loading_simulates_insert_for_load` - **Main test** that simulates exact app.rs flow (insert_task_for_load + rebuild_order)
  - `test_task_order_with_children` - Tests hierarchical ordering
  - `test_task_order_preserved_after_rebuild` - Updated to expect chronological order
  - All collapse/expand tests updated to use task_order directly
- **Result**: Single source of truth for task order, no duplicate sorting, no unused methods, consistent ordering throughout, all tests passing

### Issue 5: Missing Debug Logging ✅
- **Problem**: Agent output not logged to debug.log before processing, tool failures had no output
- **Fix**: Added comprehensive tracing
  - Log agent response immediately after generation (before tool parsing)
  - Log tool call extraction results (found X tool calls / no tools found)
  - Log tool execution start with args
  - Log tool success with result
  - **Log tool failure with ERROR level** including tool name, error message, and args
- **Files**: `crates/merlin-routing/src/agent/executor.rs:265-274, 303-314`
- **Result**: All agent output and tool execution now visible in debug.log

**Phase 1 Complete - Ready for Production Testing**

---

## Phase 2: Model Routing & Escalation
**Timeline**: 1 week
**Priority**: HIGH - Enable proper model selection across tiers

### 2.1 Fix Model Selection Between OpenRouter Models

**Problem**: Multiple OpenRouter models (DeepSeek, Qwen, etc.) not being used

**Verified Issue**: `ModelTier::Premium` has single provider field, no model variety within tier

**Solution**: Add model selection within Premium tier

**Implementation**:
1. Extend `ModelTier::Premium` to support multiple models per provider
2. Add selection logic based on task type (coding vs reasoning vs general)
3. Configure OpenRouter model preferences in config

**Example**:
```rust
// In Premium tier, select appropriate model:
match task.intent {
    TaskIntent::CodeGeneration => "deepseek-coder",
    TaskIntent::CodeReview => "claude-sonnet",
    TaskIntent::Refactoring => "qwen-coder",
    TaskIntent::Explanation => "claude-sonnet",
}
```

**Files**:
- `crates/merlin-routing/src/router/tiers.rs` - Add model selection logic
- `crates/merlin-providers/src/openrouter.rs` - Support multiple models
- `merlin-cli/merlin.toml` - Add model preferences config

### 2.2 Implement Escalation on Failure

**Problem**: No automatic escalation when model fails (Local → Groq → Premium)

**Verified Issue**: Router selects tier once, no retry logic with higher tier

**Solution**: Add escalation chain with failure detection

**Implementation**:
```rust
pub async fn execute_with_escalation(task: &Task) -> Result<Response> {
    let mut current_tier = self.router.route(task).await?.tier;
    let mut attempts = 0;
    const MAX_ATTEMPTS: usize = 3;

    loop {
        attempts += 1;
        let result = self.executor.execute(task, &current_tier).await;

        match result {
            Ok(response) if self.validator.validate(&response).await?.is_valid() => {
                return Ok(response);
            }
            Ok(_) | Err(_) if attempts < MAX_ATTEMPTS => {
                // Escalate to next tier
                current_tier = self.escalate_tier(&current_tier)?;
                tracing::warn!("Escalating to {:?} after failure", current_tier);
            }
            Err(e) => return Err(e),
            Ok(invalid) => return Err(Error::ValidationFailed(invalid)),
        }
    }
}

fn escalate_tier(&self, current: &ModelTier) -> Result<ModelTier> {
    match current {
        ModelTier::Local { .. } => Ok(ModelTier::Groq {
            model_name: "qwen2.5-32b-coder-preview".into()
        }),
        ModelTier::Groq { .. } => Ok(ModelTier::Premium {
            provider: "anthropic".into(),
            model_name: "claude-sonnet-4".into(),
        }),
        ModelTier::Premium { .. } => Err(Error::AllTiersExhausted),
    }
}
```

**Files**:
- `crates/merlin-routing/src/orchestrator.rs` - Add escalation logic
- `crates/merlin-routing/src/validator/` - Add failure detection

### 2.3 Enable Groq/Local Tier Selection

**Problem**: Groq and Local tiers not being selected appropriately

**Root Cause**: Strategies always prefer Premium for quality

**Solution**: Add cost-aware strategy that prefers lower tiers for simple tasks

**Implementation**:
1. Modify `CostOptimizationStrategy` to actually use Local/Groq for simple tasks
2. Add task classification (simple query vs complex modification)
3. Route simple tasks to Groq/Local, complex to Premium

**Example Strategy**:
```rust
impl RoutingStrategy for CostOptimizationStrategy {
    async fn select_tier(&self, task: &Task) -> Result<ModelTier> {
        match (task.complexity, task.priority) {
            (Complexity::Simple, Priority::Low | Priority::Medium) => {
                Ok(ModelTier::Local { model_name: "qwen2.5-coder:7b".into() })
            }
            (Complexity::Simple, Priority::High)
            | (Complexity::Medium, Priority::Low | Priority::Medium) => {
                Ok(ModelTier::Groq { model_name: "qwen2.5-32b-coder-preview".into() })
            }
            _ => {
                Ok(ModelTier::Premium {
                    provider: "anthropic".into(),
                    model_name: "claude-sonnet-4".into(),
                })
            }
        }
    }
}
```

**Files**:
- `crates/merlin-routing/src/router/strategies/cost.rs` - Fix strategy logic
- `crates/merlin-routing/src/analyzer/mod.rs` - Improve complexity detection

---

## Phase 3: Task Management & Verification
**Timeline**: 1 week
**Priority**: HIGH - Ensure work is tracked and verified

### 3.1 Enforce Task List Creation

**Problem**: No structured task tracking, agent doesn't break down work

**Solution**: Require task list in prompt before starting work

**Key Changes**:
```markdown
# TASK PLANNING REQUIREMENT

Before starting ANY multi-step work, you MUST create a task list:

REQUIRED FORMAT:
<tasks>
1. [Task description] - [Expected verification]
2. [Task description] - [Expected verification]
...
</tasks>

Example:
User: "Fix the bug in foo.rs"
Agent:
<tasks>
1. Read foo.rs to understand current code - Verify file loads
2. Identify the bug causing compilation error - Verify error located
3. Write fixed version of foo.rs - Verify syntax valid
4. Run cargo check to verify fix - Verify compilation succeeds
5. Run tests to ensure no regression - Verify tests pass
</tasks>

[Then execute each task in order]

DO NOT start work without a task list for multi-step operations.
```

**Implementation**:
1. Add task list requirement to prompt
2. Parse task list from response
3. Validate task list completeness before execution
4. Track task completion in TUI

**Files**:
- `prompts/coding_assistant.md` - Add task planning section
- `crates/merlin-agent/src/task_list.rs` - New task list parser
- `crates/merlin-routing/src/user_interface/task_manager.rs` - Display task list in TUI

### 3.2 Require Step Verification

**Problem**: Agent doesn't verify each step succeeded

**Solution**: Enforce verification after each significant action

**Key Changes**:
```markdown
# STEP VERIFICATION REQUIREMENT

After EVERY significant action, you MUST verify it succeeded:

ACTIONS REQUIRING VERIFICATION:
- File write → Read file back or run syntax check
- Code modification → Run cargo check
- Bug fix → Run relevant tests
- Command execution → Check exit code and output
- Feature addition → Run feature-specific tests

Example:
Agent: [writes fixed code to foo.rs]
Agent: [runs cargo check to verify syntax]
Agent: ✅ Verification: cargo check passed
Agent: [runs cargo test to verify no regression]
Agent: ✅ Verification: All tests pass

If verification fails, FIX THE ISSUE before moving to next step.
```

**Implementation**:
1. Add verification requirements to prompt
2. Add verification validator (checks for verification steps)
3. Block task completion if verification missing
4. Add verification status to TUI

**Files**:
- `prompts/coding_assistant.md` - Add verification section
- `crates/merlin-routing/src/validator/verification.rs` - New verification validator

### 3.3 Enforce Testing After Code Changes

**Problem**: Agent modifies code but doesn't test it

**Solution**: Automatic test requirement after code modifications

**Key Changes**:
```markdown
# TESTING REQUIREMENT

After ANY code modification, you MUST run appropriate tests:

CODE CHANGE → REQUIRED TEST:
- Modify lib.rs → cargo test --lib
- Modify specific module → cargo test <module_name>
- Fix bug → cargo test (all tests)
- Add feature → cargo test <feature_tests>
- Refactor → cargo test (verify no behavior change)

Example:
Agent: [modifies crates/merlin-core/src/lib.rs]
Agent: [runs cargo test --package merlin-core]
Agent: Test results: 45 passed, 0 failed
Agent: ✅ All tests pass, modification successful

If tests fail, FIX THE FAILURE before marking task complete.
```

**Implementation**:
1. Add testing requirements to prompt
2. Auto-detect code changes and require tests
3. Parse test results and report status
4. Block completion if tests fail

**Files**:
- `prompts/coding_assistant.md` - Add testing section
- `crates/merlin-routing/src/validator/testing.rs` - New testing validator
- `crates/merlin-agent/src/test_runner.rs` - Test result parser

---

## Phase 4: Continuation & Multi-Step Workflows
**Timeline**: 1 week
**Priority**: MEDIUM - Enable complex multi-step tasks

### 4.1 Add Conversation Context Tracking

**Problem**: Agent doesn't remember previous steps in conversation

**Solution**: Maintain conversation context with step history

**Implementation**:
```rust
pub struct ConversationContext {
    messages: Vec<Message>,
    completed_tasks: Vec<CompletedTask>,
    current_task_list: Option<TaskList>,
    modified_files: HashSet<PathBuf>,
    test_results: Vec<TestResult>,
}

// Include in prompt:
// "Previous steps in this conversation:
//  1. Read foo.rs - ✅ Complete
//  2. Modified foo.rs - ✅ Complete
//  3. Running verification... - 🔄 In Progress"
```

**Files**:
- `crates/merlin-routing/src/agent/conversation.rs` - Already exists, enhance with task tracking
- `prompts/coding_assistant.md` - Add conversation context section

### 4.2 Implement Multi-Turn Task Execution

**Problem**: Agent stops after one response, doesn't continue workflow

**Solution**: Enable continuation prompts and multi-turn execution

**Implementation**:
1. Add continuation detection (task list incomplete)
2. Auto-generate continuation prompt ("Continue with next step")
3. Track progress through task list
4. Stop only when all tasks complete and verified

**Example Flow**:
```
User: "Fix bug in foo.rs"
Agent: [creates task list, reads file, identifies bug, writes fix]
System: [auto-continuation: "Complete remaining verification steps"]
Agent: [runs cargo check, runs tests, reports results]
System: [detects all tasks complete, stops]
```

**Files**:
- `crates/merlin-routing/src/orchestrator.rs` - Add continuation logic
- `crates/merlin-agent/src/executor.rs` - Track task completion

### 4.3 Enable Complex TypeScript Workflows

**Problem**: Agent doesn't use TypeScript tool for complex multi-file operations

**Solution**: Add patterns to prompt showing when to use TypeScript tool

**Key Changes**:
```markdown
# WHEN TO USE TYPESCRIPT TOOL

Use TypeScript tool when you need:

1. **Loops over files**:
   - "Update all test files"
   - "Find TODOs in src/"
   - "Rename function across codebase"

2. **Conditional logic**:
   - "Fix files that have X"
   - "Update only if Y exists"
   - "Process files based on content"

3. **Data aggregation**:
   - "Count functions per file"
   - "List all public APIs"
   - "Generate statistics"

Example (Multi-file operation):
User: "Add #[must_use] to all Result-returning functions"
Agent: [uses TypeScript tool]:
```javascript
const files = await listFiles("src/**/*.rs");
let modified = 0;

for (const file of files) {
  let content = await readFile(file);
  const original = content;

  // Find functions returning Result without #[must_use]
  content = content.replace(
    /^(\s*)pub fn (\w+)\([^)]*\) -> Result</gm,
    '$1#[must_use]\n$1pub fn $2(...) -> Result<'
  );

  if (content !== original) {
    await writeFile(file, content);
    modified++;
  }
}

return `Modified ${modified} files`;
```

**Files**:
- `prompts/coding_assistant.md` - Add TypeScript patterns section

---

## Phase 5: Response Quality & Context Intelligence
**Timeline**: 2 weeks
**Priority**: MEDIUM - Polish and optimize (after agent is usable)

### 5.1 Intelligent Context Pruning

**Problem**: Too many irrelevant files in context

**Solution**: Multi-stage filtering with relevance scoring

**Implementation**:
1. Add LLM-based relevance scoring for selected files
2. Implement dependency graph expansion for imports
3. Optimize token budget allocation
4. Track context usage effectiveness

**Files**:
- `crates/merlin-context/src/builder.rs` - Add relevance scoring
- `crates/merlin-context/src/pruning.rs` - New context optimization module

### 5.2 Context Citation Enforcement

**Problem**: Agent doesn't always reference provided context

**Solution**: Add citation requirement to prompt and validator

**Implementation**:
```markdown
# CONTEXT CITATION REQUIREMENT

When referencing code, ALWAYS cite the source with line numbers:

Format: "In `file/path.rs:42`, the function..."

Example:
"The router selects tiers in `crates/merlin-routing/src/router/tiers.rs:118-150`
where it iterates through strategies and checks availability."
```

**Files**:
- `prompts/coding_assistant.md` - Add citation requirement
- `crates/merlin-routing/src/validator/citations.rs` - New citation validator

### 5.3 Dynamic Context Expansion

**Problem**: Sometimes need more context mid-conversation

**Solution**: Add tool for agent to request additional context

**Implementation**:
1. Create `requestContext(pattern, reason)` tool
2. Allow agent to request specific files mid-execution
3. Track requested files for conversation memory
4. Validate context requests aren't excessive

**Files**:
- `crates/merlin-tools/src/context_request.rs` - New context request tool
- `crates/merlin-context/src/dynamic.rs` - Dynamic context expansion

---

## Current Architecture

### What Works Today (Phases 0-5 ✅)

**9-Crate Workspace**:
- `merlin-core` - Fundamental types, traits, error handling
- `merlin-context` - Context building with semantic search (BM25 + embeddings)
- `merlin-routing` - Multi-tier model routing, task execution, validation, TUI
- `merlin-providers` - External APIs (Groq, OpenRouter, Anthropic, DeepSeek)
- `merlin-local` - Ollama integration for local models
- `merlin-agent` - Agent execution, self-assessment, streaming
- `merlin-languages` - Language backends (rust-analyzer integration)
- `merlin-tools` - File operations, command execution
- `merlin-cli` - Command-line interface and configuration

**Model Routing Pipeline**:
```
Query → Analyzer → Router → Executor → Validator → Result
  ↓         ↓         ↓         ↓          ↓
Local    Complexity  Tier     Tools    Syntax
(Qwen)   Assessment  Select   +Files   Build
  ↓         ↓         ↓         ↓       Test
Groq     Intent      Fallback Streaming Lint
(Llama)  Detection   Chain    Progress
  ↓         ↓         ↓         ↓
Premium  Subtasks   Escalate  Cache
(Claude) Parallel   Retry     Metrics
```

**Key Features**:
- ✅ Query intent classification (Conversational vs CodeQuery vs CodeModification)
- ✅ Conversation history with 50-message limit
- ✅ Multi-stage validation pipeline
- ✅ Task decomposition with 4 execution strategies
- ✅ Self-determining tasks with automatic assessment
- ✅ Response caching with semantic similarity (0.95 threshold)
- ✅ TUI with real-time progress, task trees, streaming output
- ✅ File locking and conflict detection
- ✅ Workspace isolation with snapshots
- ✅ Cost tracking and metrics collection
- ✅ TOML configuration support

### Current Performance

**Speed**:
- Local (Qwen 7B): ~100-500ms
- Groq (Llama 70B): ~500-2000ms
- Premium (Claude/DeepSeek): ~2000-5000ms

**Cost** (estimated daily):
- Local: $0 (unlimited)
- Groq: $0 (free tier: 14,400 requests/day)
- Premium: $0.50-$2.00 (depending on usage)

**Quality**:
- Test Coverage: ~35% (264 tests)
- Success Rate: ~85-90% (estimated, needs tracking)
- Escalation Rate: ~20-25% (needs reduction)

---

## Core Design Philosophy

### Guiding Principles

1. **Context is King**: The best model with wrong context fails; a weaker model with perfect context succeeds
2. **Parallel by Default**: Maximize throughput with safe concurrent execution
3. **Fail Fast, Learn Faster**: Quick validation cycles with immediate feedback
4. **Cost-Conscious Intelligence**: Use the cheapest model that can solve the problem
5. **Language-Aware**: Deep integration with language-specific tooling (LSP, analyzers)
6. **Human-Centric UX**: Real-time feedback, clear progress, easy debugging

### Inspiration from Best Agents

**Claude Code (claude.ai/code)**:
- ✅ Streaming output with real-time progress
- ✅ Tool use with structured JSON
- ❌ No local model support
- ❌ Single-threaded execution

**Cursor**:
- ✅ Fast inline completions
- ✅ Codebase-wide understanding
- ❌ Proprietary, closed-source
- ❌ Limited to IDE

**Aider**:
- ✅ Git integration
- ✅ Multi-file editing
- ❌ Limited context window management
- ❌ No parallel execution

**Devin**:
- ✅ Long-running autonomous tasks
- ✅ Browser and terminal control
- ❌ Expensive ($500/month)
- ❌ Slow iteration cycles

**Our Differentiators** (when we fix the agent):
1. **TypeScript Tool Syntax** ✅ - Natural for LLMs trained on open-source code
2. **Multi-tier Model Routing** ✅ - Cost optimization with quality fallback
3. **Semantic Context Search** ✅ - BM25 + embeddings for relevant file selection
4. **Parallel Tool Execution** ✅ - Execute independent operations concurrently
5. **Real-time TUI** ✅ - Live progress tracking and task visualization

---

## Success Metrics

### Phase 1: Action-Oriented Agent (CRITICAL)

**Must Achieve**:
- Agent calls tools directly 100% of the time (currently ~20%)
- Zero "you can use..." or "you should..." responses when action requested
- Multi-step workflows complete (not stopping after first step)
- All actions followed by verification step

**Measurement**:
- Test 20 action queries ("Read X", "Fix Y", "Run Z")
- Count advisory vs executor responses (target: 0% advisory)
- Track workflow completion rate (target: 100%)
- Verify verification steps present (target: 100%)

**Test Queries**:
1. "Read src/main.rs"
2. "Fix the bug in foo.rs"
3. "Run the tests"
4. "Add a function to calculate sum"
5. "Refactor the router module"

**Success**: Agent immediately calls tools, completes full workflow with verification

### Phase 2: Model Routing & Escalation

**Must Achieve**:
- Local/Groq tiers selected for simple tasks (currently: always Premium)
- Multiple OpenRouter models used (DeepSeek, Qwen, Claude)
- Escalation working: Local → Groq → Premium on failure
- 50% cost reduction through better tier selection

**Measurement**:
- Track tier selection by task complexity
- Monitor model variety (target: 5+ models used)
- Count escalation chains (target: <10% need escalation)
- Measure average cost per request

**Success**: Simple tasks use Local/Groq, complex use Premium, escalation works

### Phase 3: Task Management & Verification

**Must Achieve**:
- Task lists created for multi-step work (currently: none)
- Each step verified before moving forward (currently: no verification)
- Tests run after code changes (currently: never)
- Task completion tracked in TUI

**Measurement**:
- Parse task lists from responses (target: 100% for multi-step)
- Count verification steps (target: 1 per significant action)
- Test execution rate after code changes (target: 100%)
- Task completion tracking visible in TUI

**Success**: All work tracked, verified, and tested

### Phase 4: Continuation & Multi-Step Workflows

**Must Achieve**:
- Multi-turn execution for complex tasks
- Conversation context maintained across turns
- TypeScript tool used for complex multi-file operations
- Tasks complete fully, not partially

**Measurement**:
- Track average turns per complex task
- Monitor context retention across turns
- TypeScript usage rate for appropriate tasks (target: >80%)
- Full completion rate (target: 100%)

**Success**: Complex tasks execute fully across multiple turns with proper context

### Phase 5: Response Quality & Context Intelligence

**Must Achieve**:
- Context citations present (target: 80% of code references)
- Intelligent context pruning (reduce files from 18 to 5-8)
- Dynamic context requests working
- Context relevance improved

**Measurement**:
- Citation presence in responses
- Average files in context per query
- Context request usage rate
- Manual relevance scoring

**Success**: High-quality responses with proper citations and optimized context

---

## Priority Order (What to Do First)

**Week 1** - Phase 1: Make Agent Execute (CRITICAL):
1. Update `prompts/coding_assistant.md` with executor role
2. Add multi-step completion requirements
3. Add explicit tool examples
4. Test with 20 action queries

**Week 2** - Phase 2: Fix Model Routing:
1. Add model selection within Premium tier
2. Implement escalation logic
3. Fix cost optimization strategy
4. Test tier selection

**Week 3** - Phase 3: Add Task Management:
1. Enforce task list creation
2. Require step verification
3. Enforce testing after code changes
4. Update TUI to show tasks

**Week 4** - Phase 4: Enable Continuation:
1. Add conversation context tracking
2. Implement multi-turn execution
3. Add TypeScript workflow patterns
4. Test complex multi-step tasks

**Week 5-6** - Phase 5: Polish Quality:
1. Add context citation requirements
2. Implement context pruning
3. Add dynamic context requests
4. Optimize and refine

**Success Criteria for "Agent is Usable"**:
- ✅ Agent executes actions directly (no advisory responses)
- ✅ Multi-step workflows complete with verification
- ✅ Model routing works across all tiers
- ✅ Task management visible and enforced
- ✅ Tests run automatically after code changes

---

## Completed Work

### TypeScript Tool Integration ✅

**Status**: Fully implemented and integrated
- QuickJS-based runtime with sandboxing
- Tools execute synchronously using Tokio runtime
- Full JavaScript support (variables, functions, control flow, error handling)
- Integrated into orchestrator and executor pool
- System prompt includes TypeScript documentation
- 2 scenario tests passing (basic + control flow)
- 15+ unit tests passing
- Zero clippy warnings

**Files**:
- `crates/merlin-tools/src/typescript_runtime.rs` (549 lines)
- `crates/merlin-routing/src/tools/typescript.rs` (160 lines)
- `prompts/coding_assistant.md` (updated with TypeScript examples)
- `tests/fixtures/scenarios/tools/` (2 scenarios)

**Implementation Details**:
- ✅ **QuickJS-based runtime** (`merlin-tools/src/typescript_runtime.rs`)
  - Full JavaScript execution with sandboxing
  - Memory limits (64MB) and stack size limits (1MB)
  - Execution timeout (30s default, configurable)
  - Tool registration and injection into JS context
  - Type definition generation for LLM prompts
- ✅ **Routing integration** (`merlin-routing/src/tools/typescript.rs`)
  - `TypeScriptTool` wraps the runtime as a routing tool
  - `ToolWrapper` adapts routing tools to `merlin_tools::Tool` interface
  - Registered in orchestrator and executor pool with basic tools
- ✅ **Full JavaScript support**:
  - Variables, functions, arrow functions
  - Control flow (if/else, for, while loops)
  - Arrays and objects
  - Error handling (try/catch)
  - All standard JavaScript features via QuickJS
- ✅ **Comprehensive tests** (15+ tests in `typescript_runtime.rs`)
  - Runtime creation and configuration
  - Simple expressions and operations
  - Control flow and conditionals
  - Function definitions and arrow functions
  - Error handling and syntax errors
  - Type definition generation
  - Multiple tool registration

**Integration Work Completed**:
1. ✅ **Async tool execution fixed**: Tools now execute synchronously using Tokio runtime within QuickJS
2. ✅ **Scenario test coverage**: Created `tools/typescript_basic.json` and `tools/typescript_control_flow.json`
3. ✅ **Tool result handling**: Tools execute and return results properly to JavaScript context
4. ✅ **System prompt integration**: Added TypeScript tool documentation to `prompts/coding_assistant.md`
5. ✅ **Example scenarios**: UI snapshots generated and passing

**Architecture**:
```
Agent → TypeScriptTool.execute(code) → QuickJS Runtime
                                            ↓
                                    Inject tool functions (with Tokio runtime)
                                            ↓
                                    Execute JavaScript
                                            ↓
                                    Tool calls → ToolWrapper
                                            ↓
                                    Routing Tool.execute() [via block_on]
                                            ↓
                                    Return results to JS ✅ (NOW WORKING)
```

**Implementation Details**:
- Tools execute synchronously within QuickJS using `tokio::runtime::Runtime::block_on`
- Each tool function wrapper has access to a shared Tokio runtime
- Tool results are converted to JavaScript values and returned immediately
- Errors are thrown as JavaScript exceptions

**Known Limitations**:
1. **Synchronous blocking**: Tools block the QuickJS thread (acceptable for current use case)
2. **Memory leaks**: `Box::leak` used in `ToolWrapper` for 'static lifetime (acceptable for static tool names)
3. **No true async/await**: JavaScript async/await syntax works, but tools execute synchronously

**Future Enhancements** (Optional):
- Add more complex scenario tests with actual file operations
- Implement true async support using QuickJS promises and event loop
- Add timeout handling per tool call
- Add tool call logging and metrics

**Vision**: TypeScript function calls (LLMs are trained on this!)
```typescript
// Agent generates this naturally:
await readFile("src/main.rs")
await writeFile("src/lib.rs", content)
await runCommand("cargo", ["test", "--", "--nocapture"])

// Multi-step with clear intent:
const tests = await listFiles("tests/**/*.rs")
for (const test of tests) {
  const content = await readFile(test)
  if (content.includes("TODO")) {
    await writeFile(test, content.replace("TODO", "FIXED"))
  }
}
```

**Why This Approach**:
1. LLMs see millions of examples in training data
2. Natural control flow (loops, conditions, error handling)
3. Type hints guide correct parameter usage
4. Reduces hallucination (familiar syntax)
5. Enables tool chaining without special syntax

