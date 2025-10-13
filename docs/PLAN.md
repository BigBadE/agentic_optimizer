# Merlin: Next-Generation AI Coding Agent
## Comprehensive Development Roadmap

**Vision**: Build the most capable, efficient, and adaptable AI coding agent for managing large complex codebases

**Current Status**: Infrastructure Complete | 160 Rust files | 307 tests passing
**Critical Issue**: Agent produces nonsensical responses - needs fundamental improvements
**Next Phase**: 1 - Make the Agent Actually Work

---

## Table of Contents

1. [Current State & Problems](#current-state--problems)
2. [Phase 1: Core Agent Functionality](#phase-1-core-agent-functionality)
3. [Phase 2: Response Quality & Reliability](#phase-2-response-quality--reliability)
4. [Phase 3: Context Intelligence](#phase-3-context-intelligence)
5. [Phase 4: Advanced Tool Usage](#phase-4-advanced-tool-usage)
6. [Phase 5: Multi-Model Optimization](#phase-5-multi-model-optimization)
7. [Current Architecture](#current-architecture)
8. [Success Metrics](#success-metrics)

---

## Current State & Problems

### What's Actually Broken

**Critical Issues**:
1. **Agent produces nonsensical responses** - The core problem
   - Responses don't match the query intent
   - Hallucinates functions, types, and implementations
   - Ignores provided context
   - Makes up file paths and code that doesn't exist

2. **Context not being used effectively**
   - Files are fetched but agent doesn't reference them
   - BM25 + embeddings select files, but agent ignores them
   - System prompt tells agent to use context, but it doesn't

3. **Tool usage is broken**
   - Agent doesn't call tools when it should
   - When it does call tools, parameters are wrong
   - No verification that tool results are incorporated

4. **No feedback loop**
   - Agent can't see its own mistakes
   - No self-correction mechanism
   - Validation pipeline exists but doesn't improve responses

### What Actually Works

**Infrastructure** (all the plumbing is done):
- ✅ Multi-tier model routing
- ✅ Context fetching with BM25 + embeddings
- ✅ Tool registry and execution
- ✅ TUI with real-time updates
- ✅ TypeScript tool for complex workflows
- ✅ Validation pipeline
- ✅ Response caching
- ✅ Workspace isolation

**The Problem**: Great infrastructure, terrible agent behavior

---

## Phase 1: Core Agent Functionality
**Timeline**: 2-3 weeks
**Priority**: CRITICAL - Nothing else matters if the agent doesn't work

### 1.1 Fix Basic Response Quality

**Problem**: Agent ignores context and hallucinates

**Root Causes**:
1. System prompt is too generic
2. Context format not clear enough
3. Model doesn't understand what we want
4. No examples in the prompt

**Solution**: Prompt Engineering Overhaul
```rust
// New system prompt structure:
// 1. Role definition with strict constraints
// 2. Context format explanation with examples
// 3. Step-by-step reasoning requirements
// 4. Output format specification
// 5. Few-shot examples of good responses

pub fn build_system_prompt(context: &Context) -> String {
    format!(
        r#"You are a Rust coding assistant with access to the user's codebase.

CRITICAL RULES:
1. ONLY reference code that appears in the context below
2. If you don't see something in the context, say "I don't see that in the provided code"
3. Quote line numbers when referencing code (e.g., "on line 42 in src/main.rs")
4. Never guess function signatures or implementations

CONTEXT FORMAT:
Below are {} files from the codebase. Each file shows:
- Full path (e.g., crates/merlin-core/src/lib.rs)
- Complete file contents with line numbers

REASONING PROCESS:
1. First, identify which files are relevant to the query
2. Quote specific lines that answer the question
3. Explain your reasoning based on the code
4. If information is missing, explicitly state what's missing

EXAMPLE GOOD RESPONSE:
User: "How does the context builder work?"
Assistant: "Looking at crates/merlin-context/src/builder.rs, the ContextBuilder 
works by scanning the project root (line 48). It uses BM25 for keyword matching 
(line 156) and embeddings for semantic search (line 164). The build() method 
combines these scores to select the most relevant files (lines 200-215)."

EXAMPLE BAD RESPONSE:
"The context builder uses advanced AI to understand your code."
(This is too vague and doesn't reference specific code)

=== CODEBASE CONTEXT ({} files) ===
{}
=== END CONTEXT ===

Now answer the user's question using ONLY the code shown above."#,
        context.files.len(),
        context.files.len(),
        format_files_with_line_numbers(&context.files)
    )
}
```

**Implementation**:
- Rewrite `prompts/coding_assistant.md` with strict constraints
- Add few-shot examples for common query types
- Include explicit reasoning steps requirement
- Add format for citing code with line numbers

**Testing**:
- Create 20 test queries with known correct answers
- Measure hallucination rate (should be <5%)
- Measure context usage rate (should be >90%)

**Files**:
- `prompts/coding_assistant.md` - Complete rewrite
- `crates/merlin-context/src/builder.rs` - Add line numbers to context
- `crates/merlin-routing/tests/agent_quality/` - New test suite

### 1.2 Force Tool Usage

**Problem**: Agent doesn't use tools when it should

**Solution**: Tool-First Prompting
```rust
// Before answering, agent MUST:
// 1. List which tools it needs
// 2. Explain why it needs them
// 3. Call the tools
// 4. Use tool results in response

pub fn build_tool_prompt(query: &str, tools: &[Tool]) -> String {
    format!(
        r#"AVAILABLE TOOLS:
{}

MANDATORY PROCESS:
1. Analyze the query: "{}"
2. List which tools you need and why
3. Call the tools (you MUST call at least one tool if the query requires it)
4. Use the tool results in your response

Example:
Query: "What's in src/main.rs?"
Reasoning: I need to read the file to see its contents
Tool calls: readFile("src/main.rs")
Response: Based on the file contents, src/main.rs contains...

DO NOT respond without calling tools if the query requires them."#,
        format_tool_descriptions(tools),
        query
    )
}
```

**Implementation**:
- Add tool usage requirements to system prompt
- Create tool usage validator (fails if no tools called when needed)
- Add examples of correct tool usage patterns

**Testing**:
- 10 queries that require file reading
- 10 queries that require file writing
- 10 queries that require command execution
- Measure tool usage rate (should be 100% when needed)

### 1.3 Implement Chain-of-Thought Reasoning

**Problem**: Agent jumps to conclusions without reasoning

**Solution**: Require explicit reasoning steps
```rust
pub struct ReasoningResponse {
    thought_process: Vec<String>,  // Step-by-step reasoning
    evidence: Vec<CodeReference>,  // Specific code citations
    conclusion: String,            // Final answer
    confidence: f32,               // 0.0-1.0
}

// Prompt requires this structure:
// <thinking>
// 1. The query asks about...
// 2. Looking at file X, I see...
// 3. This means...
// </thinking>
// <evidence>
// - Line 42 in src/main.rs: `fn main() {`
// - Line 156 in src/lib.rs: `pub struct Context`
// </evidence>
// <answer>
// Based on the evidence above...
// </answer>
```

**Implementation**:
- Update prompt to require `<thinking>`, `<evidence>`, `<answer>` tags
- Parse and validate response structure
- Reject responses without proper reasoning

**Files**:
- `crates/merlin-agent/src/reasoning.rs` - New reasoning parser
- `crates/merlin-routing/src/validator/reasoning.rs` - Reasoning validator

### 1.4 Add Self-Correction Loop

**Problem**: Agent makes mistakes and doesn't fix them

**Solution**: Validation + Retry with Feedback
```rust
pub async fn execute_with_self_correction(
    query: &str,
    max_attempts: usize,
) -> Result<Response> {
    for attempt in 1..=max_attempts {
        let response = self.generate_response(query).await?;
        
        // Validate response
        let validation = self.validator.validate(&response).await?;
        
        if validation.is_valid() {
            return Ok(response);
        }
        
        // Give feedback and retry
        let feedback = format!(
            "Your previous response had issues:\n{}\n\
             Please try again, addressing these problems.",
            validation.issues.join("\n")
        );
        
        query = &format!("{}\n\nFEEDBACK: {}", query, feedback);
    }
    
    Err(Error::MaxAttemptsExceeded)
}
```

**Validation Checks**:
1. **Hallucination check**: All referenced code exists in context
2. **Tool usage check**: Tools called when needed
3. **Completeness check**: All parts of query addressed
4. **Format check**: Response follows required structure

**Implementation**:
- Extend validation pipeline with specific checks
- Add retry logic with feedback
- Track improvement across attempts

**Files**:
- `crates/merlin-routing/src/agent/self_correct.rs` - New module
- `crates/merlin-routing/src/validator/hallucination.rs` - Hallucination detector

---

## Phase 2: Response Quality & Reliability
**Timeline**: 2-3 weeks
**Priority**: HIGH - Make responses consistently good

### 2.1 Context Citation Enforcement

**Problem**: Agent doesn't cite sources

**Solution**: Require citations for all claims
```rust
pub struct Citation {
    file: PathBuf,
    line_start: usize,
    line_end: usize,
    quoted_text: String,
}

pub struct CitedResponse {
    answer: String,
    citations: Vec<Citation>,
}

// Validate that every claim has a citation
pub fn validate_citations(response: &str, context: &Context) -> Result<()> {
    let claims = extract_claims(response);
    for claim in claims {
        if !has_supporting_citation(claim, &response.citations, context) {
            return Err(Error::UncitedClaim(claim));
        }
    }
    Ok(())
}
```

### 2.2 Confidence Scoring

**Problem**: Agent doesn't know when it's uncertain

**Solution**: Require confidence scores
```rust
pub struct ConfidentResponse {
    answer: String,
    confidence: f32,  // 0.0 = guessing, 1.0 = certain
    reasoning: String,
    missing_info: Vec<String>,  // What would increase confidence
}

// Prompt includes:
// "Rate your confidence (0.0-1.0) based on:
// - How much relevant code you found
// - How directly it answers the question
// - Whether you had to make assumptions"
```

### 2.3 Multi-Attempt Consensus

**Problem**: Single response might be wrong

**Solution**: Generate multiple responses, pick best
```rust
pub async fn consensus_response(query: &str) -> Result<Response> {
    // Generate 3 responses with different temperatures
    let responses = vec![
        generate(query, temp=0.3).await?,
        generate(query, temp=0.5).await?,
        generate(query, temp=0.7).await?,
    ];
    
    // Score each response
    let scored: Vec<_> = responses.iter()
        .map(|r| (r, score_response(r, query)))
        .collect();
    
    // Return highest scoring
    scored.into_iter()
        .max_by_key(|(_, score)| score)
        .map(|(r, _)| r.clone())
        .ok_or(Error::NoValidResponse)
}
```

### 2.4 Response Templates

**Problem**: Inconsistent response format

**Solution**: Templates for common query types
```rust
pub enum QueryType {
    HowDoesXWork,
    WhereIsXDefined,
    WhatDoesXDo,
    HowToImplementX,
    WhyIsXBroken,
}

pub fn get_template(query_type: QueryType) -> &'static str {
    match query_type {
        QueryType::HowDoesXWork => r#"
## How {feature} Works

**Overview**: {one_sentence_summary}

**Implementation**: 
{step_by_step_explanation}

**Key Files**:
- {file1}: {purpose1}
- {file2}: {purpose2}

**Code References**:
{citations_with_line_numbers}
"#,
        // ... other templates
    }
}
```

---

## Phase 3: Context Intelligence
**Timeline**: 2-3 weeks  
**Priority**: HIGH - Better context = better responses

### 3.1 Intelligent Context Pruning

**Problem**: Too many irrelevant files in context

**Current**: BM25 + embeddings select 18 files for "remember bacon"
**Goal**: Only include truly relevant files (3-5 files max)

**Solution**: Multi-stage filtering
```rust
pub async fn select_optimal_context(
    query: &Query,
    max_tokens: usize,
) -> Vec<FileContext> {
    // Stage 1: Keyword matching (fast, broad)
    let keyword_matches = bm25.top_k(query, 50);
    
    // Stage 2: Semantic similarity (medium, precise)
    let semantic_matches = embeddings.rank(query, keyword_matches, 20);
    
    // Stage 3: Dependency analysis (slow, complete)
    let with_deps = dependency_graph.expand(semantic_matches);
    
    // Stage 4: Relevance scoring with LLM
    let scored = llm.score_relevance(query, with_deps).await?;
    
    // Stage 5: Token budget optimization
    optimize_token_budget(scored, max_tokens)
}
```

### 3.2 Dynamic Context Expansion

**Problem**: Sometimes need more context mid-conversation

**Solution**: Agent can request more files
```rust
// Agent can call: requestMoreContext(reason, file_pattern)
pub async fn handle_context_request(
    reason: &str,
    pattern: &str,
) -> Result<Vec<FileContext>> {
    info!("Agent requested more context: {}", reason);
    
    let additional_files = find_files(pattern)?;
    let validated = validate_context_request(reason, &additional_files)?;
    
    Ok(validated)
}
```

### 3.3 Conversation-Aware Context

**Problem**: Context doesn't update based on conversation

**Solution**: Track mentioned files and concepts
```rust
pub struct ConversationContext {
    mentioned_files: HashSet<PathBuf>,
    discussed_concepts: Vec<String>,
    current_focus: Option<CodeLocation>,
}

// Automatically include recently discussed files
pub fn build_context_with_history(
    query: &Query,
    history: &ConversationContext,
) -> Context {
    let mut files = select_for_query(query);
    
    // Add files from recent conversation
    files.extend(history.mentioned_files.iter().take(5));
    
    // Add files related to discussed concepts
    files.extend(find_related_to_concepts(&history.discussed_concepts));
    
    deduplicate_and_rank(files)
}
```

---

## Phase 4: Advanced Tool Usage
**Timeline**: 2-3 weeks
**Priority**: MEDIUM - Unlock complex workflows

### 4.1 Tool Chain Planning

**Problem**: Agent doesn't plan multi-step tool usage

**Solution**: Require tool execution plan
```rust
pub struct ToolPlan {
    steps: Vec<ToolStep>,
    dependencies: HashMap<StepId, Vec<StepId>>,
}

// Agent must output plan before execution:
// <tool_plan>
// 1. readFile("src/main.rs") -> get current implementation
// 2. readFile("tests/main_test.rs") -> understand requirements  
// 3. writeFile("src/main.rs", updated_content) -> apply fix
// 4. runCommand("cargo", ["test"]) -> verify fix works
// </tool_plan>
```

### 4.2 Tool Result Verification

**Problem**: Agent doesn't check if tools succeeded

**Solution**: Require verification step
```rust
// After each tool call, agent must:
// 1. Check if tool succeeded
// 2. Verify result matches expectation
// 3. Adjust plan if needed

pub async fn execute_tool_with_verification(
    tool: &dyn Tool,
    args: Value,
    expected: &str,
) -> Result<ToolOutput> {
    let result = tool.execute(args).await?;
    
    if !result.success {
        return Err(Error::ToolFailed(result.message));
    }
    
    // Agent verifies result
    let verification = verify_tool_result(&result, expected).await?;
    if !verification.matches_expectation {
        return Err(Error::UnexpectedToolResult(verification.diff));
    }
    
    Ok(result)
}
```

### 4.3 TypeScript Workflow Patterns

**Problem**: Agent doesn't know when to use TypeScript tool

**Solution**: Provide clear patterns and examples
```javascript
// Pattern 1: Batch file operations
const rustFiles = await listFiles("src/**/*.rs");
for (const file of rustFiles) {
    const content = await readFile(file);
    if (content.includes("TODO")) {
        const fixed = content.replace(/TODO:.*/g, "");
        await writeFile(file, fixed);
    }
}

// Pattern 2: Conditional workflows
const testResult = await runCommand("cargo", ["test"]);
if (testResult.code !== 0) {
    const logs = await readFile("test-output.log");
    // Analyze and fix...
}

// Pattern 3: Data aggregation
const allTests = await listFiles("tests/**/*.rs");
const testCounts = {};
for (const test of allTests) {
    const content = await readFile(test);
    const count = (content.match(/#\[test\]/g) || []).length;
    testCounts[test] = count;
}
```

---

## Phase 5: Multi-Model Optimization
**Timeline**: 2-3 weeks
**Priority**: MEDIUM - Cost and quality optimization

### 5.1 Task-Specific Model Selection

**Problem**: Using same model for all tasks is suboptimal

**Solution**: Route by task type
```rust
pub fn select_model_for_task(task: &Task) -> ModelSpec {
    match task.task_type {
        TaskType::SimpleQuery => ModelSpec::Local(Qwen7B),
        TaskType::CodeGeneration => ModelSpec::Premium(DeepSeekCoder),
        TaskType::CodeReview => ModelSpec::Premium(ClaudeSonnet),
        TaskType::Refactoring => ModelSpec::Premium(ClaudeOpus),
        TaskType::Testing => ModelSpec::Free(GroqLlama70B),
        TaskType::Documentation => ModelSpec::Free(GroqLlama70B),
    }
}
```

### 5.2 Ensemble Validation

**Problem**: Single model can make mistakes

**Solution**: Multiple models vote on correctness
```rust
pub async fn ensemble_validate(code: &str) -> ValidationResult {
    let reviews = join_all(vec![
        qwen_review(code),
        llama_review(code),
        claude_review(code),
    ]).await;
    
    let consensus = calculate_consensus(&reviews);
    if consensus.agreement >= 0.7 {
        ValidationResult::Pass(consensus)
    } else {
        ValidationResult::Uncertain(reviews)
    }
}
```

### 5.3 Adaptive Model Selection

**Problem**: Don't know which models are good at what

**Solution**: Track performance and adapt
```rust
pub struct ModelPerformance {
    model: ModelSpec,
    success_rate: HashMap<TaskType, f32>,
    avg_quality_score: HashMap<TaskType, f32>,
}

pub fn select_best_model(task_type: TaskType) -> ModelSpec {
    performance_tracker
        .get_models_for_task(task_type)
        .max_by_key(|m| m.success_rate[&task_type])
        .unwrap_or_default()
}
```

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

### Phase 1: Core Agent Functionality

**Must Achieve**:
- Hallucination rate < 5% (currently ~50%)
- Context usage rate > 90% (currently ~20%)
- Tool usage accuracy 100% when needed (currently ~30%)
- Response follows required structure 100% (currently ~10%)

**Measurement**:
- 20 test queries with known correct answers
- Automated validation of citations and context usage
- Tool call verification
- Structure parsing validation

### Phase 2: Response Quality & Reliability

**Must Achieve**:
- All claims have supporting citations (currently 0%)
- Confidence scores provided (currently N/A)
- Consistent response format (currently inconsistent)
- Multi-attempt consensus improves quality by 30%

**Measurement**:
- Citation coverage percentage
- Confidence calibration (predicted vs actual accuracy)
- Format compliance rate
- Quality improvement across attempts

### Phase 3: Context Intelligence

**Must Achieve**:
- Reduce average files in context from 18 to 3-5
- Context relevance score > 0.85 (currently ~0.60)
- Dynamic context expansion when needed
- Conversation-aware file selection

**Measurement**:
- Average files per query
- Relevance scoring by human reviewers
- Context expansion request accuracy
- Conversation continuity score

### Phase 4: Advanced Tool Usage

**Must Achieve**:
- Tool plans generated before execution
- Tool result verification 100%
- TypeScript workflow usage when appropriate
- Multi-step tool chains execute correctly

**Measurement**:
- Plan quality score
- Verification pass rate
- TypeScript usage rate for complex workflows
- Tool chain success rate

### Phase 5: Multi-Model Optimization

**Must Achieve**:
- 5x cost reduction (from $0.15 to $0.03 per request)
- Quality maintained or improved
- Model selection accuracy > 90%
- Ensemble validation reduces errors by 50%

**Measurement**:
- Average cost per request
- Quality scores by task type
- Model selection accuracy
- Error reduction with ensemble

---

## Next Actions

**Immediate (Week 1-2)**:
1. Implement Phase 1.1: Rewrite system prompt with strict constraints
2. Add line numbers to context files
3. Create 20 test queries with known answers
4. Measure baseline hallucination rate

**Short-term (Week 3-4)**:
1. Implement Phase 1.2: Force tool usage in prompts
2. Add tool usage validator
3. Implement Phase 1.3: Chain-of-thought reasoning
4. Create reasoning parser and validator

**Medium-term (Week 5-8)**:
1. Implement Phase 1.4: Self-correction loop
2. Add hallucination detector
3. Implement Phase 2.1: Citation enforcement
4. Begin Phase 2.2: Confidence scoring

**Success Criteria for "Agent Actually Works"**:
- Can answer "How does X work?" with accurate code citations
- Can perform "Read file Y and modify Z" without hallucinating
- Can execute multi-step workflows using TypeScript tool
- Responses are grounded in provided context 90%+ of the time

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

