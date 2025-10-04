# Multi-Model Routing Architecture

## Executive Summary

This document outlines a sophisticated multi-model routing system designed to optimize cost, speed, quality, and task completion. The system leverages a tiered approach combining local models, free APIs (Groq), and paid APIs (OpenRouter, Anthropic) with parallel execution for complex multi-step tasks.

**Key Goals:**
- **Cost**: 95-98% reduction through intelligent model selection
- **Quality**: Validation layers using local models for build/edit verification
- **Speed**: Local models respond in 10-200ms; parallel execution for complex workflows
- **Context**: Minimize tokens through targeted routing and task decomposition
- **Task Solving**: Structured checklists, testing, and validation pipelines
- **Parallelism**: Multi-threaded execution for independent milestones

---

## Architecture Overview

### Three-Tier Model Hierarchy

```
┌─────────────────────────────────────────────────────────────────┐
│                         User Request                             │
└────────────────────────────┬────────────────────────────────────┘
                             │
┌────────────────────────────▼────────────────────────────────────┐
│                    Request Analyzer                              │
│  • Parse intent and complexity                                   │
│  • Decompose into subtasks                                       │
│  • Identify parallelizable components                            │
│  • Estimate resource requirements                                │
└────────────────────────────┬────────────────────────────────────┘
                             │
                ┌────────────┴────────────┐
                │                         │
┌───────────────▼──────────┐   ┌──────────▼──────────────────────┐
│   Task Orchestrator      │   │   Parallel Execution Engine     │
│  • Route to model tiers  │   │  • Spawn independent threads    │
│  • Manage dependencies   │   │  • Coordinate shared state      │
│  • Queue validation      │   │  • Handle task synchronization  │
└───────────────┬──────────┘   └──────────┬──────────────────────┘
                │                         │
     ┌──────────┴──────────┐              │
     │                     │              │
┌────▼─────┐  ┌───────────▼─────┐  ┌─────▼──────┐
│  Tier 1  │  │     Tier 2      │  │   Tier 3   │
│  Local   │  │  Free Cloud     │  │ Paid Cloud │
│  Models  │  │  (Groq)         │  │ (Premium)  │
└────┬─────┘  └───────────┬─────┘  └─────┬──────┘
     │                    │               │
     └────────────────────┴───────────────┘
                          │
┌─────────────────────────▼───────────────────────────────────────┐
│                   Validation Layer                               │
│  • Local model verifies build success                            │
│  • Syntax/semantic checks on edits                               │
│  • Test execution and result parsing                             │
│  • Quality scoring and retry logic                               │
└──────────────────────────────────────────────────────────────────┘
```

---

## Tier 1: Local Models ($0 cost, 10-200ms latency)

### Model Selection

| Model | Size | VRAM | Speed | Use Cases | Cost |
|-------|------|------|-------|-----------|------|
| **Phi-3-mini** | 3.8B | 4GB | 150 tok/s | Task routing, classification | $0 |
| **Qwen2.5-Coder-7B** | 7B | 6GB | 60 tok/s | Code edits, completions | $0 |
| **Qwen2.5-Coder-14B** | 14B | 10GB | 35 tok/s | Refactoring, bug fixes | $0 |
| **Qwen2.5-Coder-32B** | 32B | 20GB | 15 tok/s | Complex analysis | $0 |
| **DeepSeek-Coder-6.7B** | 6.7B | 6GB | 50 tok/s | Build validation | $0 |

### Task Categories

**Router Model (Phi-3-mini):**
- Classification: Determine task type and complexity
- Routing: Select appropriate model tier
- Intent parsing: Extract subtasks

**Code Model (Qwen2.5-Coder):**
- Simple edits: Single-file, <50 line changes
- Code completion: Autocomplete, snippet generation
- Syntax checking: Parse and validate syntax
- Doc generation: Comments and documentation
- Test generation: Unit test scaffolding

**Validation Model (DeepSeek-Coder):**
- Build verification: Check if code compiles
- Lint analysis: Run clippy/linter checks
- Test execution: Run test suite
- Diff validation: Verify edit correctness

### Expected Load Distribution
- **Classification/Routing**: 100% (all requests)
- **Code edits**: 40% (simple, single-file)
- **Validation**: 100% (all code changes)
- **Build checks**: 100% (all compilable changes)

**Estimated Local Handling**: 40-50% of total requests

---

## Tier 2: Free Cloud APIs (Groq)

### Groq API Configuration

**Models Available:**
- **Llama 3.1 70B**: Free tier, rate-limited (~6K requests/day)
- **Llama 3.1 8B**: Free tier, higher rate limits
- **Mixtral 8x7B**: Free tier, good for structured tasks

**Pricing** (beyond free tier):
- Input: $0.59/M tokens
- Output: $0.79/M tokens

### Task Categories

**Medium Complexity, Structured Output:**
- Multi-file analysis: Analyze dependencies across files
- Refactor planning: Generate refactor strategy
- Bug diagnosis: Root cause analysis
- Code review: Review changes for issues
- Test suite generation: Create comprehensive tests

**Reasoning Tasks:**
- Architecture design: High-level system design
- Performance analysis: Bottleneck identification
- Security audit: Find vulnerabilities

### Expected Load Distribution
- **Multi-file analysis**: 20% of requests
- **Complex reasoning**: 15% of requests
- **Architecture/planning**: 10% of requests

**Estimated Groq Handling**: 25-30% of total requests (mostly within free tier)

---

## Tier 3: Paid Cloud APIs (OpenRouter, Anthropic)

### Model Selection Matrix

| Provider | Model | Input | Output | Cache Read | Best For | Quality |
|----------|-------|-------|--------|------------|----------|---------|
| **OpenRouter** | DeepSeek-V3 | $0.27/M | $1.10/M | N/A | Code-heavy | 8/10 |
| **OpenRouter** | GPT-4o | $2.50/M | $10/M | N/A | General tasks | 8.5/10 |
| **OpenRouter** | Gemini 1.5 Pro | $1.25/M | $5/M | N/A | Long context | 8/10 |
| **OpenRouter** | Claude 3.5 Sonnet | $3/M | $15/M | $0.30/M | Complex code | 9.5/10 |
| **Direct** | Claude 3 Haiku | $0.80/M | $4/M | $0.08/M | Fast iteration | 7.5/10 |
| **Direct** | Claude 3.5 Sonnet | $3/M | $15/M | $0.30/M | Critical tasks | 9.5/10 |

### Task Categories

**Complexity Requiring Top-Tier Reasoning:**
- Complex refactor: Multi-module, >500 lines
- Architecture overhaul: System-wide changes
- Algorithm optimization: Performance critical
- Critical bug fix: Production issues

**Quality-Critical:**
- Production code: Code going to main branch
- Security critical: Auth, crypto, sensitive data
- API design: Public interfaces

**Escalated Tasks:**
- When local/Groq fails or low confidence
- Retry with higher quality model

### Expected Load Distribution
- **Complex refactors**: 5% of requests
- **Architecture design**: 3% of requests
- **Critical fixes**: 2% of requests
- **Escalated tasks**: 5% of requests

**Estimated Premium Handling**: 10-15% of total requests

---

## Routing Logic: The Decision Tree

### Phase 1: Intent Analysis

```rust
struct RequestAnalyzer {
    local_classifier: Arc<RwLock<Model>>,
}

struct TaskAnalysis {
    intent: Intent,
    complexity: Complexity,
    parallelizable: Vec<Subtask>,
    context_needs: usize,
    quality_requirement: Quality,
}

enum Complexity {
    Trivial,   // <20 tokens, simple keywords
    Simple,    // <50 tokens, single file
    Medium,    // <100 tokens, multi-file
    Complex,   // >100 tokens, system-wide
}
```

**Analysis Heuristics:**
- Token count, keyword presence, file count
- Identify separators: "and", "also", "then"
- Extract dependencies between subtasks
- Estimate context window needs

### Phase 2: Model Tier Selection

**Decision Factors:**
1. **Complexity**: Trivial → Local, Complex → Premium
2. **Context Size**: >100K tokens → Gemini Pro (long context)
3. **Quality Requirement**: Critical → Claude 3.5 Sonnet
4. **Cost Optimization**: Check Groq quota first
5. **Default**: DeepSeek-V3 (cheapest premium)

**Escalation Path:**
Local → Groq → DeepSeek-V3 → Haiku → Sonnet

---

## Parallel Execution Engine

### Task Dependency Graph

Uses `petgraph` to model task dependencies:

```rust
struct TaskGraph {
    graph: DiGraph<Task, ()>,
    completed: HashSet<NodeIndex>,
}
```

**Key Operations:**
- **from_analysis**: Build graph from subtask list
- **get_ready_tasks**: Find tasks with all dependencies met
- **mark_complete**: Update completion status

### Parallel Executor

```rust
struct ParallelExecutor {
    router: Arc<ModelRouter>,
    max_concurrent: usize,
    shared_state: Arc<RwLock<WorkspaceState>>,
}
```

**Execution Flow:**
1. Get tasks with satisfied dependencies
2. Spawn up to `max_concurrent` tasks
3. Wait for completion
4. Update graph and shared state
5. Repeat until all tasks complete

### Example Workflow: Fix Build + Work on Milestone

**User Request:** "Fix all build errors, then work on milestone 3"

**Parallel Execution:**
```
Thread 1: Identify all build errors (local model, 100ms)
Thread 2: Fix error 1 in file A (Groq, 2s)
Thread 3: Fix error 2 in file B (Groq, 2s)
Thread 4: Analyze milestone 3 requirements (local model, 200ms)

After build fixed:
Thread 1: Validate build (local model, 50ms)
Thread 2-4: Work on milestone 3 subtasks in parallel
```

**Benefits:**
- **Speed**: 3 errors fixed in 2s instead of 6s sequential
- **Cost**: Validation done locally while cloud models work
- **Quality**: Each fix validated before proceeding

---

## Validation Layer

### Build Verification Pipeline

**Four-Stage Validation:**

1. **Syntax Check** (local model, 50ms)
   - Parse code with local Qwen/DeepSeek
   - Score: 0-1.0 confidence
   - Early exit if <0.8

2. **Build Check** (cargo check, 2-5s)
   - Run `cargo check` on changes
   - Capture errors/warnings
   - Use local model to diagnose failures

3. **Test Execution** (cargo test, 5-30s)
   - Run affected tests
   - Parse test output
   - Identify failures

4. **Lint Analysis** (cargo clippy, 2-5s)
   - Run clippy on changes
   - Categorize warnings by severity
   - Filter ignorable warnings

### Validation-Driven Routing

**Retry with Escalation:**
```
1. Execute on selected tier (e.g., Groq)
2. Validate output with local model
3. If validation fails:
   a. Escalate to higher tier (e.g., DeepSeek-V3)
   b. Add validation feedback to context
   c. Retry up to max_retries
4. If all retries fail, return error with diagnosis
```

**Benefits:**
- **Quality Assurance**: No broken code merged
- **Cost Efficiency**: Only escalate when necessary
- **Fast Feedback**: Local validation in <1s

---

## Context Minimization Strategies

### 1. Targeted Context Selection

**Strategy**: Only send relevant files, not entire codebase

**Implementation**:
- Extract symbols/functions from task description
- Use local index to find definitions (no API call)
- Send only relevant file excerpts (~5-20KB)
- Include minimal dependency context

**Savings**: 95% reduction in context tokens
- Before: 500KB full codebase
- After: 5-20KB targeted context

### 2. Differential Context (Multi-Turn)

**Strategy**: Track what's already been sent in conversation

**Implementation**:
- Maintain `sent_context` map per conversation
- On follow-up requests, send only new files
- Reference previous context: "As discussed, the Parser module..."

**Savings**: 80% reduction on follow-up requests
- First turn: 20KB
- Follow-ups: 4KB (only new files)

### 3. Summarized Context for High-Level Tasks

**Strategy**: For architecture/design, send summaries instead of code

**Implementation**:
- Module structure tree
- Public API signatures only
- Dependency graph
- Recent commit summaries

**Savings**: 90% reduction for high-level tasks
- Before: 500KB full code
- After: 10-20KB summaries

### 4. Streaming Context (Future)

**Strategy**: Send context incrementally as model needs it

**Implementation**:
- Model requests specific files during generation
- Stream only requested context
- Cache sent fragments for reuse

**Savings**: Pay only for used context

---

## Task Solving Enhancements

### 1. Checklist-Driven Execution

**Approach**: Break milestones into verifiable checklist items

**Flow**:
1. Generate checklist from milestone (local model, cheap)
2. For each item:
   - Execute as independent task
   - Validate completion
   - Mark complete or retry
3. Track completion rate and blockers

**Benefits**:
- **Clarity**: Clear progress tracking
- **Parallelism**: Independent items run concurrently
- **Validation**: Each item verified before proceeding

### 2. Test-First Development

**Approach**: Generate tests before implementation

**Flow**:
1. Generate tests from requirements (Groq/cheap model)
2. Validate tests compile and fail appropriately (local)
3. Implement feature to pass tests (appropriate tier)
4. Validate all tests pass (local)
5. Refine if needed

**Benefits**:
- **Quality**: Implementation validated against tests
- **Cost**: Expensive models only for implementation
- **Speed**: Tests guide implementation, fewer retries

### 3. Incremental Refinement

**Approach**: Iterative improvement with validation checkpoints

**Flow**:
1. Generate initial solution (cheap tier)
2. Validate with local model
3. If issues found:
   - Identify specific problems
   - Generate targeted fixes (keep cheap tier if possible)
   - Re-validate
4. Escalate tier only if repeated failures

**Benefits**:
- **Cost**: Start cheap, escalate only when needed
- **Quality**: Multiple validation rounds
- **Speed**: Early validation catches issues fast

### 4. Context-Aware Retries

**Approach**: Learn from failures to improve next attempt

**Flow**:
1. Execute task with minimal context
2. On failure, analyze error
3. Retry with:
   - Expanded context (add missing files)
   - Higher tier model
   - Specific failure feedback

**Benefits**:
- **Cost**: Start with minimal context
- **Quality**: Failures inform better retries
- **Success Rate**: Higher on second attempt

---

## Parallel Execution Patterns

### Pattern 1: Independent Milestones

**Scenario**: Work on multiple unrelated features

**Example**: "Add auth middleware AND implement caching"

**Execution**:
```
Thread 1: Auth middleware
  - Design API (local, 100ms)
  - Generate implementation (Groq, 3s)
  - Validate (local, 50ms)
  
Thread 2: Caching
  - Design API (local, 100ms)
  - Generate implementation (Groq, 3s)
  - Validate (local, 50ms)
```

**Time**: 3.15s parallel vs 6.3s sequential (50% faster)

### Pattern 2: Fix Build + Continue Work

**Scenario**: Build errors blocking progress

**Example**: "Fix the 3 build errors, then continue milestone 2"

**Execution**:
```
Thread 1: Fix error in parser.rs (Groq)
Thread 2: Fix error in lexer.rs (Groq)
Thread 3: Fix error in main.rs (Groq)
Thread 4: Analyze milestone 2 next steps (local)

After all build fixes:
Thread 1: Validate full build (local)
Threads 2-4: Work on milestone 2 subtasks
```

**Time**: Milestone analysis done while fixing build (overlap)

### Pattern 3: Generate + Validate Pipeline

**Scenario**: Generate code and validate in parallel

**Example**: "Refactor 5 modules"

**Execution**:
```
Thread 1: Generate refactor for module A (Groq)
Thread 2: Validate module A (local, waits for Thread 1)
Thread 3: Generate refactor for module B (Groq)
Thread 4: Validate module B (local, waits for Thread 3)
...
```

**Time**: Validation runs in parallel with next generation

### Pattern 4: Speculative Execution

**Scenario**: Prepare for likely next steps while working on current

**Example**: "Fix this bug"

**Execution**:
```
Thread 1: Diagnose and fix bug (Groq)
Thread 2 (speculative): Generate tests for bug (local)
Thread 3 (speculative): Prepare documentation update (local)

If fix succeeds:
  Apply tests and docs immediately
If fix fails:
  Discard speculative work, retry fix
```

**Time**: No extra time if fix succeeds (prepared in parallel)

---

## Cost Projections

### Current State (All Sonnet 3.5)
```
Daily:   $15.20
Monthly: $456.00
Yearly:  $5,472.00
```

### With Smart Routing (No Local)
```
Task Distribution:
- 30% Groq (free): $0.00/day
- 50% DeepSeek-V3: $0.15/day
- 15% Haiku: $0.50/day
- 5% Sonnet: $0.75/day

Daily:   $1.40
Monthly: $42.00
Yearly:  $504.00
Savings: 91% reduction ($414/month)
```

### With Full Hybrid (Local + Cloud)
```
Task Distribution:
- 40% Local models: $0.00/day (electricity: ~$0.005/day)
- 25% Groq (free): $0.00/day
- 20% DeepSeek-V3: $0.12/day
- 10% Haiku: $0.30/day
- 5% Sonnet: $0.75/day

Daily:   $0.57
Monthly: $17.10
Yearly:  $205.20
Savings: 96% reduction ($438/month)
```

### With Parallel Execution Benefits
```
Additional Savings:
- 30% faster task completion → less API time
- 50% fewer retries (better validation)
- Reuse validation results across parallel tasks

Estimated Additional Savings: 10-15%
Final Daily Cost: $0.48 - $0.51
Final Monthly Cost: $14.40 - $15.30
Total Savings: 97% reduction ($441/month)
```

---

## Implementation Phases

### Phase 1: Foundation (Week 1)

**Goals:**
- Implement local model manager (Ollama integration)
- Build task analyzer and complexity classifier
- Create model router with Groq integration

**Deliverables:**
- Local models: Phi-3-mini, Qwen2.5-Coder-7B, DeepSeek-Coder-6.7B
- Basic routing: Local → Groq → OpenRouter
- Cost tracking infrastructure

**Expected Savings:** 70-80% reduction

### Phase 2: Validation Pipeline (Week 2)

**Goals:**
- Implement build verification system
- Add syntax/lint checking with local models
- Create validation-driven retry logic

**Deliverables:**
- Four-stage validation pipeline
- Local model validation (pre-build)
- Escalation on validation failure

**Expected Savings:** Additional 5-10% (fewer failed generations)

### Phase 3: Context Optimization (Week 3)

**Goals:**
- Build codebase indexer
- Implement targeted context selection
- Add differential context for multi-turn

**Deliverables:**
- Symbol table and dependency graph
- Context builder (<20KB per request)
- Conversation state tracking

**Expected Savings:** Additional 5-10% (smaller contexts)

### Phase 4: Parallel Execution (Week 4)

**Goals:**
- Implement task dependency graph
- Build parallel executor with shared state
- Add subtask decomposition

**Deliverables:**
- Parallel task spawning (max 4 concurrent)
- Dependency management
- Shared workspace state

**Expected Benefits:** 2-3x faster for multi-step tasks

### Phase 5: Advanced Patterns (Week 5)

**Goals:**
- Add checklist-driven execution
- Implement test-first development
- Create speculative execution

**Deliverables:**
- Milestone → checklist generator
- Test generation before implementation
- Speculative work for common patterns

**Expected Benefits:** Higher quality, fewer retries

---

## Success Metrics

### Cost Metrics
- **Daily API cost < $1.00** (93% reduction)
- **Monthly cost < $30** (93% reduction)
- **Cost per request < $0.05** (was $0.50)
- **Free tier usage**: 50-60% of cloud requests

### Performance Metrics
- **Median response time < 500ms** (local models)
- **Complex task time reduction**: 2-3x with parallelism
- **Build validation time < 5s** (local + cargo check)

### Quality Metrics
- **Success rate > 95%** (first attempt)
- **Build failure rate < 5%** (after validation)
- **Escalation rate < 15%** (local/cheap → premium)

### Efficiency Metrics
- **Context size < 20KB** (was 500KB)
- **Local handling rate**: 40-50%
- **Free tier handling rate**: 25-30%
- **Premium tier usage**: 10-15%

---

## Risk Mitigation

### Risk 1: Local Model Quality Issues
**Mitigation**: Conservative routing, always validate, quick escalation
**Fallback**: Skip local tier if repeated failures
**Impact**: Low (validation catches issues)

### Risk 2: Groq Rate Limits
**Mitigation**: Track quota, fallback to DeepSeek-V3 automatically
**Fallback**: Use paid tier within free tier exhausted
**Impact**: Medium (affects 25% of requests)

### Risk 3: Parallel Execution Conflicts
**Mitigation**: Shared state locking, dependency tracking
**Fallback**: Sequential execution for conflicting tasks
**Impact**: Low (most tasks independent)

### Risk 4: Validation Overhead
**Mitigation**: Parallel validation, cache results, skip for non-code
**Fallback**: Optional validation via flag
**Impact**: Low (validation faster than regeneration)

---

## Future Enhancements

### 1. Dynamic Model Selection
Learn from past successes: which models handle which tasks best

### 2. Cost-Quality Tradeoffs
User-configurable: prefer speed, cost, or quality

### 3. Streaming Execution
Start work before full request analyzed (progressive refinement)

### 4. Multi-Agent Collaboration
Multiple models debate solutions, vote on best approach

### 5. Custom Model Fine-Tuning
Fine-tune local models on successful generations

---

## Appendix: Configuration

### Hardware Requirements

**Minimum** (Local validation only):
- 8GB RAM (CPU inference)
- No GPU required
- Handles: Classification, validation

**Recommended** (Local + simple code gen):
- 16GB VRAM (RTX 4060 Ti) OR 32GB RAM
- Handles: All local tasks, 40% of requests

**Optimal** (Maximum local usage):
- 24GB+ VRAM (RTX 4090) OR 64GB RAM
- Run Qwen2.5-Coder-32B for near-Sonnet quality
- Handles: 50-60% of requests locally

### Model Setup

```bash
# Install Ollama
# Windows: Download from ollama.ai

# Pull models
ollama pull phi3:mini              # 3.8B router
ollama pull qwen2.5-coder:7b       # 7B code specialist
ollama pull deepseek-coder:6.7b    # 6.7B validator

# Optional: larger models
ollama pull qwen2.5-coder:14b      # 14B for better quality
ollama pull qwen2.5-coder:32b      # 32B for complex tasks
```

### API Keys Required

```toml
[providers]
groq_api_key = "..."           # Free tier: 6K requests/day
openrouter_api_key = "..."     # Paid tier for DeepSeek/GPT/Gemini
anthropic_api_key = "..."      # Optional: direct Claude access
```

### Configuration File

```toml
[routing]
max_concurrent_tasks = 4
enable_parallel_execution = true
enable_speculative_execution = false

[local_models]
router = "phi3:mini"
coder = "qwen2.5-coder:7b"
validator = "deepseek-coder:6.7b"

[tiers]
# Tier preferences by task type
simple_edit = ["local", "groq", "deepseek"]
refactor = ["groq", "deepseek", "haiku"]
architecture = ["groq", "sonnet"]
critical = ["sonnet"]

[validation]
enable_syntax_check = true
enable_build_check = true
enable_test_execution = true
enable_lint_check = true

[context]
max_context_tokens = 20000
enable_differential_context = true
enable_summarized_context = true

[cost]
daily_budget_usd = 5.00
warn_threshold_usd = 4.00
stop_threshold_usd = 6.00
```

---

**End of Plan**

*Implementation Timeline: 5 weeks*
*Target Cost Reduction: 96-97% ($440+/month savings)*
*Target Performance: 2-3x faster with parallelism*
*Quality: 95%+ success rate with validation*
